#!/usr/bin/env bash

corn examples/config.corn -t json > examples/config.json
corn examples/config.corn -t toml > examples/config.toml
corn examples/config.corn -t yaml > examples/config.yaml