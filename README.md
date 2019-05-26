# Smeagol

Smeagol is a simple personal wiki using Git as a backend. This allows external modification just as
if it is a normal repository.

## Setup

Smeagol is written in Rust. It can be built and ran using `cargo`. All dependencies should be
installed automatically. The binary requires the `static/` and `template/` directories in its
working directory.

It can also be built using Docker ([Dockerfile](Dockerfile)). To be able to access the repository
externally you can mount it to a directory and add a [`USER`
directive](https://docs.docker.com/engine/reference/builder/#user) to the Dockerfile to prevent
permission problems.

## Configuration

Smeagol can be configured using [Smeagol.toml](Smeagol.toml). The default configuration is for a
debug build and only allows local access to the server. This can be changed using `bind =
"0.0.0.0:8000"`.

