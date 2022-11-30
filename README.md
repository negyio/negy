<h1 align="center">
  <img src="https://user-images.githubusercontent.com/3483230/200106390-5d7184c3-8b58-4304-92a2-2f18d7c4edaa.png" width="250"/>
</h1>

<p align="center">
  <a href="https://github.com/negyio/negy/actions/workflows/cd.yml"><img src="https://github.com/negyio/negy/actions/workflows/cd.yml/badge.svg"/></a>
  <a href="https://github.com/negyio/negy/actions/workflows/cd-dev.yml"><img src="https://github.com/negyio/negy/actions/workflows/cd-dev.yml/badge.svg"/></a>
  <a href="https://hub.docker.com/repository/docker/tbrand/negy-gateway"><img src="https://img.shields.io/docker/pulls/tbrand/negy-gateway"/></a>
  <a href="https://hub.docker.com/repository/docker/tbrand/negy-node"><img src="https://img.shields.io/docker/pulls/tbrand/negy-node"/></a>
  <a href="https://hub.docker.com/repository/docker/tbrand/negy-node-pool"><img src="https://img.shields.io/docker/pulls/tbrand/negy-node-pool"/></a>
  <a href="https://negy.io"><img src="https://img.shields.io/badge/Docs-negy.io-green"/></a>
<p align="center">

**Negy is a L4 proxy that defends your privacy transparently. It's following Tor protocol but not compatible. You can try Negy by the command below.**

```bash
# Sorry, currently this command would not work since public endpoint is closed.
# If you're interested in Negy, try to build your own network.
# => See https://negy.io/docs/contribution/development_contribution

curl https://example.com -x http://gateway.negy.io
```

## Features

- :white_check_mark: Secure http tunneling as L4 proxy following Tor protocol
- :white_check_mark: Stick to network layer. It doesn't interfere with the UX of the application
- :white_check_mark: Dark web is not our target. Everybody can aquire the secure routing
- :white_check_mark: Written in Rust

Want to know about Negy? Visit [negy.io](https://negy.io)!

## Contribution

User contributions are needed to stabilise the network. Users who can afford public computing resources are encouraged to join the Negy network. You can join with a single command. [Here](https://negy.io/docs/contribution/launch_public_node) describes how to do it.
