Expose Private Net http(behind NAT gateway) to Public

Current focuses in HTTP

# TODO
2. Fix PING PONG
   3. if client disconnected, server should close those port
3. Update error messages
4. simple error should not fail
5. ROAD to TLS
   1. client to server (tls)
   2. server to client (tls)
   3. Allow client via tls certificate auth
6. Look if, we want to expose localnet under tls?
7. Currently http as a protocol is not exposed
   1. exposing only http protocol allows 
      1. maintaining history
      2. easy way to debug issues from server
