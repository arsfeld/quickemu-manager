use binrw::io::Cursor;
use binrw::{BinRead, BinWrite};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use spice_client::protocol::*;

fn benchmark_header_serialization(c: &mut Criterion) {
    let header = SpiceDataHeader {
        serial: 12345,
        msg_type: 101,
        msg_size: 1024,
        sub_list: 0,
    };

    c.bench_function("serialize SpiceDataHeader", |b| {
        b.iter(|| {
            let mut cursor = Cursor::new(Vec::new());
            header.write_le(&mut cursor).unwrap();
            cursor.into_inner()
        });
    });

    let mut cursor = Cursor::new(Vec::new());
    header.write_le(&mut cursor).unwrap();
    let serialized = cursor.into_inner();

    c.bench_function("deserialize SpiceDataHeader", |b| {
        b.iter(|| {
            let mut cursor = Cursor::new(black_box(&serialized));
            let _: SpiceDataHeader = SpiceDataHeader::read_le(&mut cursor).unwrap();
        });
    });
}

fn benchmark_link_message_serialization(c: &mut Criterion) {
    let link_mess = SpiceLinkMess {
        connection_id: 42,
        channel_type: ChannelType::Display as u8,
        channel_id: 0,
        num_common_caps: 5,
        num_channel_caps: 10,
        caps_offset: 64,
    };

    c.bench_function("serialize SpiceLinkMess", |b| {
        b.iter(|| {
            let mut cursor = Cursor::new(Vec::new());
            link_mess.write_le(&mut cursor).unwrap();
            cursor.into_inner()
        });
    });

    let mut cursor = Cursor::new(Vec::new());
    link_mess.write_le(&mut cursor).unwrap();
    let serialized = cursor.into_inner();

    c.bench_function("deserialize SpiceLinkMess", |b| {
        b.iter(|| {
            let mut cursor = Cursor::new(black_box(&serialized));
            let _: SpiceLinkMess = SpiceLinkMess::read_le(&mut cursor).unwrap();
        });
    });
}

fn benchmark_channel_type_conversion(c: &mut Criterion) {
    c.bench_function("ChannelType from u8", |b| {
        b.iter(|| {
            for i in 0..20u8 {
                let _ = ChannelType::from(black_box(i));
            }
        });
    });
}

criterion_group!(
    benches,
    benchmark_header_serialization,
    benchmark_link_message_serialization,
    benchmark_channel_type_conversion
);
criterion_main!(benches);
