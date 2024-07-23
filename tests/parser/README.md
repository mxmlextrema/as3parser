# Parsing tests

To test parsing a program producing output to the command line, run:

```
cargo run --bin sunform_as3parser_test -- --source-path tests/parser/Demo.as
```

To test parsing a program producing output to two files `.ast.json` and `.diag`, run:

```
cargo run --bin sunform_as3parser_test -- --source-path tests/parser/Demo.as --file-log
```

For parsing MXML, pass the `--mxml` flag.

For parsing CSS, pass the `--css` flag.