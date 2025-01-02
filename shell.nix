{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell { buildInputs = with pkgs; [ stdenv gnumake git cargo-expand probe-rs ]; }
