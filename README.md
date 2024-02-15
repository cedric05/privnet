Expose Private Net http(behind NAT gateway) to Public

Current focuses in HTTP

# TODO

- [x] Fix PING PONG
   - [x] if client disconnected, server should close those port
- [x] Update error messages
- [x] simple error should not fail
- [ ] Look if, we want to expose localnet under tls?
  - [ ] ROAD to TLS
     - [ ] client to server (tls)
     - [ ] server to client (tls)
     - [ ] Allow client via tls certificate auth
     - [ ] use certificates from client
- [ ] Currently http as a protocol is not exposed
   - [ ] exposing only http protocol allows 
      - [ ] maintaining history
      - [ ] easy way to debug issues from server
- [x] Single port server (where client requests port, when supplied from client)
