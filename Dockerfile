# Gaden: closed-loop hunger-circuit simulation. Simulation only. Not a medical device.
#   docker build -t gaden .
#   docker run --rm gaden                 # one nominal day
#   docker run --rm gaden suite 50 pid    # 50 scenarios
#   docker run --rm gaden metrics         # full 1,000-scenario benchmark (takes minutes)
FROM rust:1-slim AS build
WORKDIR /src
COPY rust/Cargo.toml rust/Cargo.lock ./rust/
COPY rust/src ./rust/src
COPY rust/examples ./rust/examples
COPY rust/tests ./rust/tests
COPY data ./data
WORKDIR /src/rust
RUN cargo build --release --locked --bin gaden --bin metrics

FROM debian:stable-slim
COPY --from=build /src/rust/target/release/gaden /usr/local/bin/gaden
COPY --from=build /src/rust/target/release/metrics /usr/local/bin/metrics
ENTRYPOINT ["gaden"]
CMD ["day"]
