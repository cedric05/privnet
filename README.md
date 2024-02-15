Expose Private Net http(behind NAT gateway) to Public

Current focuses in HTTP

# TODO

- [ ] Fix PING PONG
   - [ ] if client disconnected, server should close those port
- [ ] Update error messages
- [ ] simple error should not fail
- [ ] ROAD to TLS
   - [ ] client to server (tls)
   - [ ] server to client (tls)
   - [ ] Allow client via tls certificate auth
- [ ] Look if, we want to expose localnet under tls?
- [ ] Currently http as a protocol is not exposed
   - [ ] exposing only http protocol allows 
      - [ ] maintaining history
      - [ ] easy way to debug issues from server
- [ ] Single port server (where client requests port, when supplied from client)
- [ ] use certificates from client
