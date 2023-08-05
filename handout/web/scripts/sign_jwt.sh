#!/bin/bash

#
# JWT Encoder Bash Script
#
# Usage: ./jwt_encoder.sh <id> <access_level>
#

# Check if id and access_level are provided as arguments
if [ $# -ne 2 ]; then
  echo "Usage: ./jwt_encoder.sh <id> <access_level>"
  exit 1
fi

# Extract id and access_level from arguments
id=$1
access_level=$2

# Static header fields.
header='{"alg": "HS384"}'

# Use jq to set the dynamic `iat` and `exp`
# fields on the header using the current time.
# `iat` is set to now, and `exp` is now + 1 second.
header=$(
  echo "${header}" | jq --arg time_str "$(date +%s)" \
  '
  ($time_str | tonumber) as $time_num
  | .iat=$time_num
  | .exp=($time_num + 1)
  '
)

payload="{\"id\": \"$id\",\"access_level\": \"$access_level\"}"

base64_encode()
{
  # Use `base64` directly on binary data, then URL encode the result.
  base64 | tr -d '=' | tr '/+' '_-' | tr -d '\r\n'
}

hmacsha256_sign()
{
  declare input=${1:-$(</dev/stdin)}
  printf '%s' "${input}" | openssl dgst -sha384 -hmac "${JWT_SECRET}" -binary
}

json() {
  declare input=${1:-$(</dev/stdin)}
  printf '%s' "${input}" | jq -c .
}

header_base64=$(echo "${header}" | json | base64_encode)
payload_base64=$(echo "${payload}" | json | base64_encode)

header_payload=$(echo "${header_base64}.${payload_base64}")
signature=$(echo "${header_payload}" | hmacsha256_sign | base64_encode)

echo "${header_payload}.${signature}"
