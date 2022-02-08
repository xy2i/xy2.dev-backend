# xy2.dev backend

## Setup

### Dependencies

Install rust. You need  

1. Install postgresql and setup an user and a database. After setting up an user, your connection
string will be like `postgresql://admin:admin@localhost`, with username and password to replace. 
2. Put this variable as `DATABASE_URL` in a `.env` file at the root of the project:
```
DATABASE_URL=postgresql://admin:admin@localhost
```
3. Install sqlx cli: `cargo install sqlx-cli`, providing the `sqlx` binary.
4. Run the database migrations: `sqlx migrate run --database-url=postgresql://admin:admin@localhost`
5. Run the app: `cargo run`

### Set logging

Run the app with `RUST_LOG` variable set, for example: `RUST_LOG="debug" cargo run`. 
`RUST_LOG` can be set on a more precise level, [see the doc for env_logger](https://docs.rs/env_logger/latest/env_logger/).
