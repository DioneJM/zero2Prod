# Using the latest rust stable release
# builder stage
FROM rust:1.56.0 AS chef

# Cargo chef is a tool that lets you build and cache project dependencies
RUN cargo install cargo-chef
# Go into directory `app`
#(or create directory if it doesn't exist then enter it)
WORKDIR /app
RUN apt update && apt install lld clang -y

# Copy all files working environment to Docker image
FROM chef AS planner
COPY . .
# Create a lock file for the project dependencies
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Build only project dependencies
RUN RUST_BACKTRACE=1 cargo chef cook --release --recipe-path recipe.json
# Cache results - the above will only re-run if the project's dependencies change
COPY . .
# Use offline sqlx mode
ENV SQLX_OFFLINE true

# Build the release binary
ENV APP_ENVIRONMENT production
RUN cargo build --release
# end of builder stage

# runtime stage
# We use a `slim` version to minimise the size of the final docker image
FROM rust:1.56.0-slim AS runtime
WORKDIR /app
# Copy the compiled binary created from the builder stage
COPY --from=builder /app/target/release/zero2prod zero2prod
COPY configuration configuration
ENV APP_ENVIRONMENT production
# Execute the binary
# This gets called when `docker run` is executed
CMD ["ls"]
ENTRYPOINT ["./zero2prod"]