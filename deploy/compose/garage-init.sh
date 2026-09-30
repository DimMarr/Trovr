#!/bin/sh
# One-shot, idempotent setup of the single-node Garage used by docker-compose:
# cluster layout, the API's access key, the bucket, and the CORS rule browsers
# need to PUT/GET presigned URLs from the web origin.
set -eu
apk add --no-cache curl jq openssl >/dev/null

ADMIN="http://garage:3903"
S3="http://garage:3900"
BUCKET="${S3_BUCKET:-trovr}"
ORIGIN="${WEB_ORIGIN:-http://localhost:8080}"

admin() {
  method=$1 path=$2
  shift 2
  curl -sS --fail-with-body -X "$method" -H "Authorization: Bearer $GARAGE_ADMIN_TOKEN" "$ADMIN$path" "$@"
}

echo "waiting for garage"
until admin GET /v2/GetClusterStatus >/dev/null 2>&1; do sleep 1; done

if [ "$(admin GET /v2/GetClusterLayout | jq '.roles | length')" = "0" ]; then
  echo "assigning the single-node layout"
  node=$(admin GET /v2/GetClusterStatus | jq -r '.nodes[0].id')
  admin POST /v2/UpdateClusterLayout \
    -d "{\"roles\":[{\"id\":\"$node\",\"zone\":\"dc1\",\"capacity\":10000000000,\"tags\":[]}]}" >/dev/null
  version=$(admin GET /v2/GetClusterLayout | jq '.version + 1')
  admin POST /v2/ApplyClusterLayout -d "{\"version\":$version}" >/dev/null
fi

echo "waiting for the cluster to be healthy"
until [ "$(admin GET /v2/GetClusterHealth | jq -r .status)" = "healthy" ]; do sleep 1; done

if ! admin GET "/v2/GetKeyInfo?id=$S3_ACCESS_KEY_ID" >/dev/null 2>&1; then
  echo "importing the API's access key"
  admin POST /v2/ImportKey \
    -d "{\"accessKeyId\":\"$S3_ACCESS_KEY_ID\",\"secretAccessKey\":\"$S3_SECRET_ACCESS_KEY\",\"name\":\"trovr\"}" >/dev/null
fi

# Checked apart from reading the id: `sh` has no pipefail to report curl's failure.
if admin GET "/v2/GetBucketInfo?globalAlias=$BUCKET" >/tmp/bucket.json 2>/dev/null; then
  bucket=$(jq -r .id /tmp/bucket.json)
else
  echo "creating bucket $BUCKET"
  admin POST /v2/CreateBucket -d "{\"globalAlias\":\"$BUCKET\"}" >/tmp/bucket.json
  bucket=$(jq -r .id /tmp/bucket.json)
fi
admin POST /v2/AllowBucketKey \
  -d "{\"bucketId\":\"$bucket\",\"accessKeyId\":\"$S3_ACCESS_KEY_ID\",\"permissions\":{\"read\":true,\"write\":true,\"owner\":true}}" >/dev/null

echo "allowing $ORIGIN to GET/PUT objects (CORS)"
cors="<CORSConfiguration><CORSRule><AllowedOrigin>$ORIGIN</AllowedOrigin><AllowedMethod>GET</AllowedMethod><AllowedMethod>PUT</AllowedMethod><AllowedHeader>content-type</AllowedHeader><MaxAgeSeconds>3600</MaxAgeSeconds></CORSRule></CORSConfiguration>"
md5=$(printf '%s' "$cors" | openssl dgst -md5 -binary | base64)
curl -fsS -X PUT "$S3/$BUCKET?cors" \
  --aws-sigv4 "aws:amz:garage:s3" --user "$S3_ACCESS_KEY_ID:$S3_SECRET_ACCESS_KEY" \
  -H "Content-MD5: $md5" -H "Content-Type: application/xml" --data-binary "$cors"

echo "garage is ready"
