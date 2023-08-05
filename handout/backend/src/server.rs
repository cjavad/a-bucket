use a_http_parser::http::{Method, MimeType};
use a_http_parser::parser::Parser;
use a_http_parser::request::Request;
use a_http_parser::response::Response;
use tokio_stream::StreamExt;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Mutex;

use crate::authentication::{AuthContext, AuthLevel};
use crate::metadata::Metadata;
use crate::storable::{StorableBlob, StorableJson};
use crate::storage::{Storage, Object};

pub struct Server {
    address: String,
    listener: TcpListener,
}

impl Server {
    pub async fn new(address: &str) -> Self {
        let listener = match TcpListener::bind(address).await {
            Ok(listener) => {
                println!("Listening on {}", address);
                listener
            }
            Err(error) => {
                panic!("{}: local port couldn't be bound - {}", address, error);
            }
        };

        Self {
            address: address.to_string(),
            listener,
        }
    }

    pub async fn run(self) {
        let (tx, rx) = mpsc::channel::<Arc<Mutex<Conn>>>(100);

        let manager_handle = tokio::spawn(Self::manager_loop(rx));
        let accept_handle = tokio::spawn(self.accept_loop(tx));

        let _ = tokio::try_join!(manager_handle, accept_handle);
    }

    pub async fn manager_loop(mut rx: Receiver<Arc<Mutex<Conn>>>) {
        let mut connections: Vec<Arc<Mutex<Conn>>> = Vec::new();

        while let Some(conn) = rx.recv().await {
            connections.push(conn);

            // Here you can manage connections, send broadcast messages, etc.
            // every time a client connects cleanup authentication sessions
            // that are older than 30 minutes

            if let Ok(mut list) = AuthContext::list().await {
                'outer: while let Some(auth_context) = list.next().await.unwrap() {
                    // Older than 30 minutes
                    if SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64
                        - auth_context.last_used as i64
                        > 1800 as i64
                    // Is not an admin
                    && auth_context.access_level < AuthLevel::Admin
                    // Is not the owner of any metadata
                    {
                        let mut list = Metadata::list().await.unwrap();

                        while let Some(metadata) = list.next().await.unwrap() {
                            if metadata.owner_id == auth_context.access_key {
                                continue 'outer;
                            }
                        }

                        match auth_context.delete().await {
                            Ok(_) => println!("{}: deleted auth context", auth_context.access_key),
                            Err(_) => println!(
                                "{}: failed to delete auth context",
                                auth_context.access_key
                            ),
                        };
                    }
                }
            }
        }
    }

    pub async fn accept_loop(self, tx: Sender<Arc<Mutex<Conn>>>) {
        loop {
            match self.listener.accept().await {
                Err(error) => {
                    println!("{}: something bad happened - {}", self.address, error);
                }
                Ok((socket, address)) => {
                    let conn = Arc::new(Mutex::new(Conn::new(socket, address).await));
                    let conn_clone = Arc::clone(&conn);

                    tokio::spawn(async move {
                        conn_clone.lock().await.run().await;
                        // If you want to remove finished connections from the list,
                        // you could send a message here using another channel.
                    });

                    tx.send(conn).await.unwrap();
                }
            }
        }
    }
}

pub struct Conn {
    address: SocketAddr,
    socket: TcpStream,
}

impl Conn {
    async fn new(socket: TcpStream, address: SocketAddr) -> Self {
        Self { address, socket }
    }

    async fn close(&mut self) {
        if let Err(e) = self.socket.shutdown().await {
            println!("Failed to shutdown socket: {}", e);
        }
    }

    async fn handle_storage(
        req: Request,
        res: &mut Response,
        obj: &mut Option<Object>,
        auth_context: AuthContext,
    ) -> () {
        let storage = Storage::new(auth_context.clone());
        let key = req.uri.trim_start_matches('/');

        if key.len() == 0 && req.method != Method::LIST && req.method != Method::TRACE {
            res.set_status_code(400);
            res.set_body("Bad request".as_bytes().to_vec(), MimeType::TextPlain);
            return;
        }

        match req.method {
            Method::GET => {
                if let Some(object) = storage.get_object(key, false).await {
                    if object.metadata.readable_by != AuthLevel::Public {
                        res.mark_required_authentication();
                    }

                    if req.headers.get("if-none-match").is_some() {
                        if req.headers.get("if-none-match").unwrap() == &object.metadata.etag {
                            res.set_status_code(304);
                            res.set_body("Not modified".as_bytes().to_vec(), MimeType::TextPlain);
                            return;
                        }
                    }

                    if req.headers.get("if-modified-since").is_some() {
                        if req.headers.get("if-modified-since").unwrap()
                            == &object.metadata.last_modified.to_string()
                        {
                            res.set_status_code(304);
                            res.set_body("Not modified".as_bytes().to_vec(), MimeType::TextPlain);
                            return;
                        }
                    }

                    if let Some(accept) = req.headers.get("accept") {
                        if accept == "application/octet-stream" {
                            res.set_header(
                                "Content-Disposition",
                                &format!("attachment; filename=\"{}\"", object.metadata.name),
                            );
                        }
                    }

                    res.set_status_code(200);

                    res.set_header("last-modified", &object.metadata.last_modified.to_string());
                    res.set_header("etag", &object.metadata.etag);

                    res.set_header("content-length", &format!("{}", &object.get_file_size().await.unwrap()));
                    res.set_header("content-type", &object.metadata.mime_type);

                    let _ = obj.insert(object);
                } else {
                    res.set_status_code(404);
                    res.set_body(
                        "Not found / Forbidden".as_bytes().to_vec(),
                        MimeType::TextPlain,
                    );
                }
            }
            Method::PUT | Method::POST => {
                res.mark_required_authentication();

                // This is a CDN so we should make it public by default
                let mut readable_by = AuthLevel::Public;

                // Use header X-Readable-By to set read access for other users
                if req.headers.contains_key("x-readable-by") {
                    match req.headers.get("x-readable-by").unwrap().as_str() {
                        "Owner" => readable_by = AuthLevel::Owner,
                        "Read" => readable_by = AuthLevel::Read,
                        "Public" => readable_by = AuthLevel::Public,
                        _ => (),
                    }
                }

                if readable_by != AuthLevel::Public
                    && auth_context.access_level < AuthLevel::ReadWrite
                {
                    res.set_status_code(403);
                    res.set_body(
                        "Forbidden to write non public files".as_bytes().to_vec(),
                        MimeType::TextPlain,
                    );
                    return;
                }

                if storage.put_object(
                    key,
                    &req.raw_body,
                    req.mime_type.unwrap_or_default(),
                    readable_by,
                ).await {
                    res.set_status_code(200);
                } else {
                    res.set_status_code(400);
                    res.set_body("Failed to save".as_bytes().to_vec(), MimeType::TextPlain);
                }
            }
            Method::DELETE => {
                res.mark_required_authentication();

                if auth_context.access_level < AuthLevel::ReadWrite {
                    res.set_status_code(403);
                    res.set_body("Forbidden".as_bytes().to_vec(), MimeType::TextPlain);
                    return;
                }

                if storage.delete_object(key).await {
                    res.set_status_code(200);
                } else {
                    res.set_status_code(400);
                    res.set_body("Failed to delete".as_bytes().to_vec(), MimeType::TextPlain);
                }
            }
            Method::HEAD => {
                if let Some(object) = storage.get_object(key, false).await {
                    if object.metadata.readable_by != AuthLevel::Public {
                        res.mark_required_authentication();
                    }

                    res.set_status_code(200);
                    res.set_header(
                        "Content-Disposition",
                        &format!("attachment; filename=\"{}\"", object.metadata.name),
                    );
                    res.set_header("Last-Modified", &object.metadata.last_modified.to_string());
                    res.set_header("Etag", &object.metadata.etag);
                    res.set_header("Content-Type", &object.metadata.mime_type);
                    res.set_header("Content-Length", &object.metadata.size.to_string());
                } else {
                    res.set_status_code(404);
                    res.set_body("Not Found".as_bytes().to_vec(), MimeType::TextPlain);
                }
            }
            Method::LIST | Method::TRACE => {
                res.mark_required_authentication();

                // list objects in JSON
                let objects = storage.list_objects().await;
                // Use serde to convert the vector to a JSON string.
                let json = serde_json::to_string(&objects).unwrap();

                res.set_status_code(200);
                res.set_body(json.as_bytes().to_vec(), MimeType::ApplicationJson);
            }
            _ => {}
        }

        auth_context.to_owned().update_last_used();
        match auth_context.save().await {
            Ok(_) => {}
            Err(_) => {}
        };

    }

    async fn handle_http_request(parser: Parser) -> (Response, Option<Object>) {
        let mut res = Response::new(200);
        let mut obj: Option<Object> = None;

        if parser.is_invalid() {
            res.set_status_code(400);
            res.set_body(
                "Invalid HTTP Request".as_bytes().to_vec(),
                MimeType::TextPlain,
            );

            return (res, obj);
        }

        let request: Request = parser.consume_request().unwrap();
        let cookies: std::collections::HashMap<String, String> =
            request.cookies.clone().unwrap_or_default();

        let mut auth_context: Option<AuthContext> = None;
        let token_str = cookies.get("authorization");

        match token_str {
            Some(token) => match AuthContext::id_from_jwt(token) {
                (Some(id), Some(level)) => {
                    auth_context = match AuthContext::load(&id).await {
                        Ok(mut context) => {
                            // Only update the access level if it's lower than the current one never downgrade
                            if level > context.access_level {
                                context.access_level = level;
                                match context.save().await {
                                    Ok(_) => {}
                                    Err(_) => {}
                                }
                            }

                            Some(context)
                        }
                        Err(_) => None,
                    };
                }
                (Some(id), None) => {
                    auth_context = match AuthContext::load(&id).await {
                        Ok(context) => Some(context),
                        Err(_) => None,
                    };
                }
                _ => {
                    res.set_status_code(400);
                    res.set_body(
                        "Invalid Authorization Token".as_bytes().to_vec(),
                        MimeType::TextPlain,
                    );

                    return (res, obj);
                }
            },
            None => {}
        }

        if auth_context.is_none() {
            auth_context = Some(AuthContext::random());

            let context = auth_context.unwrap();

            match context.save().await {
                Ok(_) => {
                    res.set_cookie("authorization", context.to_owned().as_jwt().as_str(), true);
                }
                Err(_) => {
                    res.set_status_code(503);
                    res.set_body(
                        "Internal Server Error".as_bytes().to_vec(),
                        MimeType::TextPlain,
                    );

                    return (res, obj);
                }
            };
            Self::handle_storage(request, &mut res, &mut obj, context).await;
        } else {
            Self::handle_storage(request, &mut res, &mut obj, auth_context.unwrap()).await;
        }

        (res, obj)
    }

    async fn run(&mut self) {
        let mut buffer = [0; 1024];
        let (mut reader, mut writer) = self.socket.split();

        let mut parser: Parser = Parser::new();

        loop {
            match reader.read(&mut buffer).await {
                Err(error) => {
                    println!("{}: something bad happened - {}", self.address, error);
                }

                Ok(count) if count == 0 => {
                    break println!("{0}: end of stream", self.address);
                }

                Ok(count) => {
                    parser.update(&buffer[..count]);
                    if parser.is_done() {
                        break;
                    }
                }
            }
        }

        let (response, object) = Self::handle_http_request(parser).await;

        writer.write_all(&response.as_bytes()).await.unwrap();
        writer.flush().await.unwrap();

        if let Some(obj) = object {
            if let Ok(mut iterator) = obj.stream_file().await {
                while let Some(Ok(chunk)) = iterator.next().await {
                    writer.write_all(&chunk).await.unwrap();
                    writer.flush().await.unwrap();
                }
            }
        }

        self.close().await;
    }
}
