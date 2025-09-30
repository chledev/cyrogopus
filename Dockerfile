# Build Stage
FROM alpine:latest AS builder
WORKDIR /build
USER root

# Install needed binaries and tools
RUN apk add --no-cache bash coreutils curl btrfs-progs xfsprogs-extra zfs restic

# Environment and helper
ENV TO_GATHER="df,curl,btrfs,xfs_quota,zfs,uname,restic"
ENV OUTPUT_DIR="/build/gathered"
COPY .docker/helpers/gather.sh /usr/local/bin/gather
RUN chmod +x /usr/local/bin/gather && /usr/local/bin/gather

# Run Stage
FROM alpine:latest

# Copy gathered binaries and libs
COPY --from=builder /build/gathered/ /

# Add cyrogopus and entrypoint
ARG TARGETPLATFORM
COPY .docker/${TARGETPLATFORM#linux/}/cyrogopus /usr/bin/cyrogopus

ENTRYPOINT ["/usr/bin/cyrogopus"]
