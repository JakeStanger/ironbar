#!/usr/bin/env bash

shopt -s extglob

for dir in examples/*;
do
  echo "$dir"
  corn "$dir/config.corn" -t json > "$dir/config.json"
  corn "$dir/config.corn" -t toml > "$dir/config.toml"
  corn "$dir/config.corn" -t yaml > "$dir/config.yaml"
done

