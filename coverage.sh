#!/bin/sh
cargo tarpaulin -o html
open tarpaulin-report.html
