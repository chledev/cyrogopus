use super::{CompressionLevel, CompressionType};
use gzp::ZWriter;
use std::{
    io::Write,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::AsyncWrite;

pub enum CompressionWriter<'a, W: Write + Send + 'static> {
    None(W),
    Gz(gzp::par::compress::ParCompress<gzp::deflate::Gzip>),
    Xz(Box<lzma_rust2::XzWriterMt<W>>),
    Bz2(bzip2::write::BzEncoder<W>),
    Lz4(lz4::Encoder<W>),
    Zstd(zstd::Encoder<'a, W>),
}

impl<'a, W: Write + Send + 'static> CompressionWriter<'a, W> {
    pub fn new(
        writer: W,
        compression_type: CompressionType,
        compression_level: CompressionLevel,
        threads: usize,
    ) -> std::io::Result<Self> {
        match compression_type {
            CompressionType::None => Ok(CompressionWriter::None(writer)),
            CompressionType::Gz => {
                let builder = gzp::par::compress::ParCompressBuilder::new()
                    .num_threads(threads)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to configure Gzip threads: {}", e)))?
                    .compression_level(gzp::Compression::new(compression_level.to_deflate_level()))
                    .from_writer(writer);
                Ok(CompressionWriter::Gz(builder))
            }
            CompressionType::Xz => {
                let mut options = lzma_rust2::XzOptions::with_preset(match compression_level {
                    CompressionLevel::BestSpeed => 1,
                    CompressionLevel::GoodSpeed => 4,
                    CompressionLevel::GoodCompression => 6,
                    CompressionLevel::BestCompression => 9,
                });
                if let Some(block_size) = std::num::NonZeroU64::new(1 << 20) {
                    options.set_block_size(Some(block_size));
                }

                let xz_writer = lzma_rust2::XzWriterMt::new(writer, options, threads as u32)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to create XZ encoder: {}", e)))?;
                Ok(CompressionWriter::Xz(Box::new(xz_writer)))
            }
            CompressionType::Bz2 => Ok(CompressionWriter::Bz2(bzip2::write::BzEncoder::new(
                writer,
                bzip2::Compression::new(match compression_level {
                    CompressionLevel::BestSpeed => 1,
                    CompressionLevel::GoodSpeed => 4,
                    CompressionLevel::GoodCompression => 6,
                    CompressionLevel::BestCompression => 9,
                }),
            ))),
            CompressionType::Lz4 => {
                lz4::EncoderBuilder::new()
                    .build(writer)
                    .map(CompressionWriter::Lz4)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to create LZ4 encoder: {}", e)))
            }
            CompressionType::Zstd => {
                let mut encoder = zstd::Encoder::new(
                    writer,
                    match compression_level {
                        CompressionLevel::BestSpeed => 1,
                        CompressionLevel::GoodSpeed => 8,
                        CompressionLevel::GoodCompression => 14,
                        CompressionLevel::BestCompression => 22,
                    },
                )
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to create Zstd encoder: {}", e)))?;
                
                if let Err(e) = encoder.multithread(threads as u32) {
                    tracing::warn!("Failed to enable Zstd multithreading: {}", e);
                }

                Ok(CompressionWriter::Zstd(encoder))
            }
        }
    }

    pub fn finish(self) -> std::io::Result<()> {
        match self {
            CompressionWriter::None(mut writer) => writer.flush(),
            CompressionWriter::Gz(mut writer) => writer.finish().map_err(std::io::Error::other),
            CompressionWriter::Xz(writer) => {
                writer.finish()?;
                Ok(())
            }
            CompressionWriter::Bz2(writer) => {
                writer.finish()?;
                Ok(())
            }
            CompressionWriter::Lz4(writer) => {
                let (_, result) = writer.finish();

                result?;
                Ok(())
            }
            CompressionWriter::Zstd(writer) => {
                writer.finish()?;
                Ok(())
            }
        }
    }
}

impl<'a, R: Write + Send + 'static> Write for CompressionWriter<'a, R> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            CompressionWriter::None(writer) => writer.write(buf),
            CompressionWriter::Gz(writer) => writer.write(buf),
            CompressionWriter::Xz(writer) => writer.write(buf),
            CompressionWriter::Bz2(writer) => writer.write(buf),
            CompressionWriter::Lz4(writer) => writer.write(buf),
            CompressionWriter::Zstd(writer) => writer.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            CompressionWriter::None(writer) => writer.flush(),
            CompressionWriter::Gz(writer) => writer.flush(),
            CompressionWriter::Xz(writer) => writer.flush(),
            CompressionWriter::Bz2(writer) => writer.flush(),
            CompressionWriter::Lz4(writer) => writer.flush(),
            CompressionWriter::Zstd(writer) => writer.flush(),
        }
    }
}

pub struct AsyncCompressionWriter {
    inner_error_receiver: tokio::sync::oneshot::Receiver<std::io::Error>,
    inner_writer: tokio::io::DuplexStream,
}

impl AsyncCompressionWriter {
    pub fn new(
        writer: impl Write + Send + 'static,
        compression_type: CompressionType,
        compression_level: CompressionLevel,
        threads: usize,
    ) -> Self {
        let (inner_reader, inner_writer) = tokio::io::duplex(crate::BUFFER_SIZE * 4);
        let (inner_error_sender, inner_error_receiver) = tokio::sync::oneshot::channel();

        tokio::task::spawn_blocking(move || {
            let mut reader = tokio_util::io::SyncIoBridge::new(inner_reader);
            let mut stream = match CompressionWriter::new(writer, compression_type, compression_level, threads) {
                Ok(stream) => stream,
                Err(e) => {
                    let _ = inner_error_sender.send(e);
                    return;
                }
            };

            match crate::io::copy(&mut reader, &mut stream) {
                Ok(_) => {}
                Err(e) => {
                    let _ = inner_error_sender.send(e);
                    return;
                }
            }

            match stream.finish() {
                Ok(_) => {}
                Err(e) => {
                    let _ = inner_error_sender.send(e);
                }
            }
        });

        Self {
            inner_error_receiver,
            inner_writer,
        }
    }
}

impl AsyncWrite for AsyncCompressionWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        if !self.inner_error_receiver.is_terminated()
            && let Poll::Ready(result) = Pin::new(&mut self.inner_error_receiver).poll(cx)
            && let Ok(err) = result
        {
            return Poll::Ready(Err(err));
        }

        Pin::new(&mut self.inner_writer).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        if !self.inner_error_receiver.is_terminated()
            && let Poll::Ready(result) = Pin::new(&mut self.inner_error_receiver).poll(cx)
            && let Ok(err) = result
        {
            return Poll::Ready(Err(err));
        }

        Pin::new(&mut self.inner_writer).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        if !self.inner_error_receiver.is_terminated()
            && let Poll::Ready(result) = Pin::new(&mut self.inner_error_receiver).poll(cx)
            && let Ok(err) = result
        {
            return Poll::Ready(Err(err));
        }

        Pin::new(&mut self.inner_writer).poll_shutdown(cx)
    }
}
