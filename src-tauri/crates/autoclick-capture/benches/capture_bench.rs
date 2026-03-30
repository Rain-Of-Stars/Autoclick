use std::{sync::Arc, thread, time::Duration};

use autoclick_capture::{
    CaptureError, CaptureFactory, CaptureSharedState, RunningCapture, WgcCaptureOptions,
    frame::{FramePacket, PixelFormat},
    latest_frame::LatestFrameBuffer,
    preview_encode::{PreviewEncodeOptions, encode_preview},
    session::{CaptureSession, CaptureSessionConfig, CaptureTarget},
    single_frame::capture_single_frame_with_session,
};
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};

struct FakeRunner;

impl RunningCapture for FakeRunner {
    fn stop(&mut self) -> Result<(), CaptureError> {
        Ok(())
    }

    fn is_finished(&self) -> bool {
        false
    }
}

struct FakeFactory;

impl CaptureFactory for FakeFactory {
    fn start_window(
        &self,
        _hwnd: isize,
        _options: &WgcCaptureOptions,
        shared: Arc<CaptureSharedState>,
    ) -> Result<Box<dyn RunningCapture>, CaptureError> {
        thread::spawn(move || {
            shared.publish_frame(640, 360, PixelFormat::Gray8, vec![32; 640 * 360]);
        });
        Ok(Box::new(FakeRunner))
    }

    fn start_monitor(
        &self,
        _monitor_handle: Option<isize>,
        _options: &WgcCaptureOptions,
        shared: Arc<CaptureSharedState>,
    ) -> Result<Box<dyn RunningCapture>, CaptureError> {
        thread::spawn(move || {
            shared.publish_frame(1920, 1080, PixelFormat::Bgra8, vec![128; 1920 * 1080 * 4]);
        });
        Ok(Box::new(FakeRunner))
    }
}

fn bench_latest_buffer(c: &mut Criterion) {
    let latest = LatestFrameBuffer::new();
    let frame = FramePacket {
        frame_id: 1,
        width: 1920,
        height: 1080,
        pixel_format: PixelFormat::Bgra8,
        timestamp_ms: 1,
        bytes: vec![64; 1920 * 1080 * 4],
    };

    let mut group = c.benchmark_group("capture_publish");
    group.throughput(Throughput::Bytes(frame.bytes.len() as u64));
    group.bench_function("window_like_publish_and_read", |b| {
        b.iter(|| {
            latest.publish(black_box(frame.clone()));
            black_box(latest.read_latest().expect("latest"));
        });
    });
    group.finish();
}

fn bench_single_frame(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_frame");
    group.bench_function("session_single_frame_window", |b| {
        b.iter(|| {
            let mut session = CaptureSession::with_factory(Arc::new(FakeFactory));
            let frame = capture_single_frame_with_session(
                &mut session,
                CaptureSessionConfig {
                    target: CaptureTarget::Window { hwnd: 100 },
                    options: WgcCaptureOptions::default(),
                },
                Duration::from_secs(1),
            )
            .expect("single frame");
            black_box(frame);
        });
    });
    group.finish();
}

fn bench_preview_encode(c: &mut Criterion) {
    let frame = FramePacket {
        frame_id: 1,
        width: 1280,
        height: 720,
        pixel_format: PixelFormat::Bgra8,
        timestamp_ms: 1,
        bytes: vec![200; 1280 * 720 * 4],
    };
    let options = PreviewEncodeOptions::default();

    let mut group = c.benchmark_group("preview_encode");
    group.throughput(Throughput::Bytes(frame.bytes.len() as u64));
    group.bench_function("png_thumbnail_encode", |b| {
        b.iter(|| {
            let encoded = encode_preview(black_box(&frame), black_box(&options)).expect("encode");
            black_box(encoded.bytes.len());
        });
    });
    group.finish();
}

criterion_group!(
    capture_bench,
    bench_latest_buffer,
    bench_single_frame,
    bench_preview_encode
);
criterion_main!(capture_bench);
