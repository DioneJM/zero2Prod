# Using the latest rust stable release
# builder stage
FROM rust:1.56.0 AS builder

# Go into directory `app`
#(or create directory if it doesn't exist then enter it)
WORKDIR /app

# Copy all files working environment to Docker image
COPY . .

# Use offline sqlx mode
ENV SQLX_OFFLINE true

# Build the release binary
ENV APP_ENVIRONMENT production
RUN cargo build --release
# end of builder stage

# runtime stage
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