#!/bin/bash
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
ENDPOINT="http://localhost/cdn/"

curl -X LIST $ENDPOINT -c cookie.txt -v

for file in $DIR/../data/*; do
    content_type=$(file -b --mime-type $file)
    curl -b cookie.txt -X POST $ENDPOINT$(basename $file) --data-binary "@$file" -H "Content-Type: $content_type" > /dev/null
done

JWT=$(cat cookie.txt | grep "authorization" | cut -f 7)
rm cookie.txt

DECODED_JSON=$($DIR/decode_jwt.sh $JWT)

ID=$(echo $DECODED_JSON | jq -r '.id')

curl -b "authorization=$($DIR/sign_jwt.sh $ID Admin)" -X PUT ${ENDPOINT}the/flag/is/well/hidden/$(echo $FLAG | sha256sum | cut -d' ' -f1).txt --data "Here is the $FLAG, find it if you can :)" -H "X-Readable-By: Owner"

# Exit successfully
exit 0