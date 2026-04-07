# STEADFAST

The STEADFAST stack is my personal web stack. The goal of this stack is to
have a dependency free implementation of everything that I normally need for web
and application development. This library contains a wide variety of tools:

- [ ] Async Runtime (created mvp)
- [ ] HTTP implementation
  - [x] 1.1 (created mvp)
  - [ ] 2.0
  - [ ] Server (created mvp)
    - [ ] SSE
  - [x] Routing
    - [x] Request extractor abstractions
- [ ] Serialization macros
  - [x] JSON
  - [ ] HTTP
    - [ ] Body
      - [ ] URI Query Params
      - [ ] Multipart Form
    - [x] Query
      - [x] URI Query Params
- [ ] HTTPS/TLS implementation
- [ ] Custom fork of the fixi.js implementation (created mvp)
  - [ ] SSE support
- [x] HTML macro with fx attributes as first-class attributes
- [ ] Relational Database (spec still in developement)
  - [ ] Global guid search system
  - [ ] Rust as schema system
    - [ ] struct+enum versioning/hash system
- [ ] Low code macro system

