cargo install diesel_cli --no-default-features --features "sqlite-bundled"

mkdir tmp
diesel --database-url tmp/print-schema.sqlite setup
diesel --database-url tmp/print-schema.sqlite migration run
rmdir /q /s tmp