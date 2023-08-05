rm -f *.zip

zip -r handout.zip handout/
# Generate slightly modified docker compose file for the handout
# Add .env.example as .env in the handout zip file

mv .env .env.bak
mv .env.example .env
zip -u handout.zip .env
mv .env .env.example
mv .env.bak .env

cp handout.zip handout/web/data/source_code.zip

zip -r a-bucket.zip . -x a-bucket.zip -x solve.sh -x generate_handout.sh -x handout.zip
zip -u a-bucket.zip handout/web/data/source_code.zip -x a-bucket.zip -x solve.sh -x generate_handout.sh -x handout.zip

rm handout/web/data/source_code.zip
