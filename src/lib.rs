use futures_lite::{AsyncRead, AsyncWrite};
use log::trace;
use std::{
    future::Future,
    io::{self, Cursor, Read, Write},
    pin::Pin,
    task::{Context, Poll},
};
use trillium::async_trait;
use trillium_server_common::{Connector, Transport};
use trillium_smol::spawn;
use url::Url;

/// A JSON request body.
pub static REQUEST_BODY: [u8; 453] = [
    0x7b, 0x22, 0x70, 0x65, 0x65, 0x72, 0x5f, 0x61, 0x67, 0x67, 0x72, 0x65, 0x67, 0x61, 0x74, 0x6f,
    0x72, 0x5f, 0x65, 0x6e, 0x64, 0x70, 0x6f, 0x69, 0x6e, 0x74, 0x22, 0x3a, 0x22, 0x68, 0x74, 0x74,
    0x70, 0x3a, 0x2f, 0x2f, 0x61, 0x67, 0x67, 0x72, 0x65, 0x67, 0x61, 0x74, 0x6f, 0x72, 0x2e, 0x6b,
    0x69, 0x6e, 0x64, 0x2d, 0x63, 0x69, 0x2d, 0x63, 0x61, 0x73, 0x74, 0x6f, 0x72, 0x2e, 0x73, 0x76,
    0x63, 0x2e, 0x63, 0x6c, 0x75, 0x73, 0x74, 0x65, 0x72, 0x2e, 0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x2f,
    0x22, 0x2c, 0x22, 0x71, 0x75, 0x65, 0x72, 0x79, 0x5f, 0x74, 0x79, 0x70, 0x65, 0x22, 0x3a, 0x22,
    0x54, 0x69, 0x6d, 0x65, 0x49, 0x6e, 0x74, 0x65, 0x72, 0x76, 0x61, 0x6c, 0x22, 0x2c, 0x22, 0x76,
    0x64, 0x61, 0x66, 0x22, 0x3a, 0x22, 0x50, 0x72, 0x69, 0x6f, 0x33, 0x43, 0x6f, 0x75, 0x6e, 0x74,
    0x22, 0x2c, 0x22, 0x72, 0x6f, 0x6c, 0x65, 0x22, 0x3a, 0x22, 0x48, 0x65, 0x6c, 0x70, 0x65, 0x72,
    0x22, 0x2c, 0x22, 0x6d, 0x61, 0x78, 0x5f, 0x62, 0x61, 0x74, 0x63, 0x68, 0x5f, 0x71, 0x75, 0x65,
    0x72, 0x79, 0x5f, 0x63, 0x6f, 0x75, 0x6e, 0x74, 0x22, 0x3a, 0x31, 0x2c, 0x22, 0x74, 0x61, 0x73,
    0x6b, 0x5f, 0x65, 0x78, 0x70, 0x69, 0x72, 0x61, 0x74, 0x69, 0x6f, 0x6e, 0x22, 0x3a, 0x31, 0x37,
    0x33, 0x32, 0x30, 0x37, 0x30, 0x36, 0x35, 0x31, 0x2c, 0x22, 0x6d, 0x69, 0x6e, 0x5f, 0x62, 0x61,
    0x74, 0x63, 0x68, 0x5f, 0x73, 0x69, 0x7a, 0x65, 0x22, 0x3a, 0x31, 0x30, 0x30, 0x2c, 0x22, 0x74,
    0x69, 0x6d, 0x65, 0x5f, 0x70, 0x72, 0x65, 0x63, 0x69, 0x73, 0x69, 0x6f, 0x6e, 0x22, 0x3a, 0x32,
    0x38, 0x38, 0x30, 0x30, 0x2c, 0x22, 0x63, 0x6f, 0x6c, 0x6c, 0x65, 0x63, 0x74, 0x6f, 0x72, 0x5f,
    0x68, 0x70, 0x6b, 0x65, 0x5f, 0x63, 0x6f, 0x6e, 0x66, 0x69, 0x67, 0x22, 0x3a, 0x7b, 0x22, 0x69,
    0x64, 0x22, 0x3a, 0x36, 0x38, 0x2c, 0x22, 0x6b, 0x65, 0x6d, 0x5f, 0x69, 0x64, 0x22, 0x3a, 0x22,
    0x58, 0x32, 0x35, 0x35, 0x31, 0x39, 0x48, 0x6b, 0x64, 0x66, 0x53, 0x68, 0x61, 0x32, 0x35, 0x36,
    0x22, 0x2c, 0x22, 0x6b, 0x64, 0x66, 0x5f, 0x69, 0x64, 0x22, 0x3a, 0x22, 0x48, 0x6b, 0x64, 0x66,
    0x53, 0x68, 0x61, 0x32, 0x35, 0x36, 0x22, 0x2c, 0x22, 0x61, 0x65, 0x61, 0x64, 0x5f, 0x69, 0x64,
    0x22, 0x3a, 0x22, 0x41, 0x65, 0x73, 0x31, 0x32, 0x38, 0x47, 0x63, 0x6d, 0x22, 0x2c, 0x22, 0x70,
    0x75, 0x62, 0x6c, 0x69, 0x63, 0x5f, 0x6b, 0x65, 0x79, 0x22, 0x3a, 0x22, 0x66, 0x5f, 0x73, 0x49,
    0x64, 0x36, 0x54, 0x6d, 0x59, 0x75, 0x33, 0x48, 0x4b, 0x47, 0x47, 0x2d, 0x4b, 0x42, 0x53, 0x79,
    0x67, 0x50, 0x50, 0x2d, 0x5a, 0x33, 0x73, 0x62, 0x38, 0x57, 0x73, 0x65, 0x75, 0x65, 0x57, 0x64,
    0x55, 0x77, 0x4c, 0x43, 0x49, 0x6c, 0x4d, 0x22, 0x7d, 0x2c, 0x22, 0x76, 0x64, 0x61, 0x66, 0x5f,
    0x76, 0x65, 0x72, 0x69, 0x66, 0x79, 0x5f, 0x6b, 0x65, 0x79, 0x22, 0x3a, 0x22, 0x55, 0x45, 0x63,
    0x70, 0x4b, 0x4f, 0x50, 0x59, 0x51, 0x65, 0x73, 0x66, 0x5a, 0x59, 0x33, 0x66, 0x77, 0x74, 0x32,
    0x33, 0x74, 0x77, 0x22, 0x7d,
];
/// Three packets of HTTP response frames, concatenated together, with status codes 100, 100, and
/// 200.
pub static CANNED_RESPONSE_BODY: [u8; 1054] = [
    0x48, 0x54, 0x54, 0x50, 0x2f, 0x31, 0x2e, 0x31, 0x20, 0x31, 0x30, 0x30, 0x20, 0x43, 0x6f, 0x6e,
    0x74, 0x69, 0x6e, 0x75, 0x65, 0x0d, 0x0a, 0x0d, 0x0a, 0x48, 0x54, 0x54, 0x50, 0x2f, 0x31, 0x2e,
    0x31, 0x20, 0x31, 0x30, 0x30, 0x20, 0x43, 0x6f, 0x6e, 0x74, 0x69, 0x6e, 0x75, 0x65, 0x0d, 0x0a,
    0x53, 0x65, 0x72, 0x76, 0x65, 0x72, 0x3a, 0x20, 0x43, 0x61, 0x64, 0x64, 0x79, 0x0d, 0x0a, 0x0d,
    0x0a, 0x48, 0x54, 0x54, 0x50, 0x2f, 0x31, 0x2e, 0x31, 0x20, 0x32, 0x30, 0x30, 0x20, 0x4f, 0x4b,
    0x0d, 0x0a, 0x43, 0x6f, 0x6e, 0x74, 0x65, 0x6e, 0x74, 0x2d, 0x4c, 0x65, 0x6e, 0x67, 0x74, 0x68,
    0x3a, 0x20, 0x38, 0x30, 0x38, 0x0d, 0x0a, 0x43, 0x6f, 0x6e, 0x74, 0x65, 0x6e, 0x74, 0x2d, 0x54,
    0x79, 0x70, 0x65, 0x3a, 0x20, 0x61, 0x70, 0x70, 0x6c, 0x69, 0x63, 0x61, 0x74, 0x69, 0x6f, 0x6e,
    0x2f, 0x76, 0x6e, 0x64, 0x2e, 0x6a, 0x61, 0x6e, 0x75, 0x73, 0x2e, 0x61, 0x67, 0x67, 0x72, 0x65,
    0x67, 0x61, 0x74, 0x6f, 0x72, 0x2b, 0x6a, 0x73, 0x6f, 0x6e, 0x3b, 0x76, 0x65, 0x72, 0x73, 0x69,
    0x6f, 0x6e, 0x3d, 0x30, 0x2e, 0x31, 0x0d, 0x0a, 0x44, 0x61, 0x74, 0x65, 0x3a, 0x20, 0x54, 0x75,
    0x65, 0x2c, 0x20, 0x32, 0x31, 0x20, 0x4e, 0x6f, 0x76, 0x20, 0x32, 0x30, 0x32, 0x33, 0x20, 0x30,
    0x32, 0x3a, 0x34, 0x34, 0x3a, 0x31, 0x31, 0x20, 0x47, 0x4d, 0x54, 0x0d, 0x0a, 0x53, 0x65, 0x72,
    0x76, 0x65, 0x72, 0x3a, 0x20, 0x43, 0x61, 0x64, 0x64, 0x79, 0x0d, 0x0a, 0x53, 0x65, 0x72, 0x76,
    0x65, 0x72, 0x3a, 0x20, 0x74, 0x72, 0x69, 0x6c, 0x6c, 0x69, 0x75, 0x6d, 0x2f, 0x30, 0x2e, 0x33,
    0x2e, 0x35, 0x0d, 0x0a, 0x0d, 0x0a, 0x7b, 0x22, 0x74, 0x61, 0x73, 0x6b, 0x5f, 0x69, 0x64, 0x22,
    0x3a, 0x22, 0x75, 0x46, 0x68, 0x34, 0x61, 0x4c, 0x4b, 0x32, 0x6f, 0x65, 0x51, 0x47, 0x63, 0x53,
    0x35, 0x6c, 0x4e, 0x73, 0x39, 0x4d, 0x50, 0x32, 0x48, 0x74, 0x6e, 0x79, 0x4e, 0x46, 0x58, 0x30,
    0x5f, 0x76, 0x68, 0x58, 0x4e, 0x38, 0x4b, 0x2d, 0x6d, 0x61, 0x76, 0x38, 0x51, 0x22, 0x2c, 0x22,
    0x70, 0x65, 0x65, 0x72, 0x5f, 0x61, 0x67, 0x67, 0x72, 0x65, 0x67, 0x61, 0x74, 0x6f, 0x72, 0x5f,
    0x65, 0x6e, 0x64, 0x70, 0x6f, 0x69, 0x6e, 0x74, 0x22, 0x3a, 0x22, 0x68, 0x74, 0x74, 0x70, 0x3a,
    0x2f, 0x2f, 0x61, 0x67, 0x67, 0x72, 0x65, 0x67, 0x61, 0x74, 0x6f, 0x72, 0x2e, 0x6b, 0x69, 0x6e,
    0x64, 0x2d, 0x63, 0x69, 0x2d, 0x63, 0x61, 0x73, 0x74, 0x6f, 0x72, 0x2e, 0x73, 0x76, 0x63, 0x2e,
    0x63, 0x6c, 0x75, 0x73, 0x74, 0x65, 0x72, 0x2e, 0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x2f, 0x22, 0x2c,
    0x22, 0x71, 0x75, 0x65, 0x72, 0x79, 0x5f, 0x74, 0x79, 0x70, 0x65, 0x22, 0x3a, 0x22, 0x54, 0x69,
    0x6d, 0x65, 0x49, 0x6e, 0x74, 0x65, 0x72, 0x76, 0x61, 0x6c, 0x22, 0x2c, 0x22, 0x76, 0x64, 0x61,
    0x66, 0x22, 0x3a, 0x22, 0x50, 0x72, 0x69, 0x6f, 0x33, 0x43, 0x6f, 0x75, 0x6e, 0x74, 0x22, 0x2c,
    0x22, 0x72, 0x6f, 0x6c, 0x65, 0x22, 0x3a, 0x22, 0x48, 0x65, 0x6c, 0x70, 0x65, 0x72, 0x22, 0x2c,
    0x22, 0x76, 0x64, 0x61, 0x66, 0x5f, 0x76, 0x65, 0x72, 0x69, 0x66, 0x79, 0x5f, 0x6b, 0x65, 0x79,
    0x22, 0x3a, 0x22, 0x55, 0x45, 0x63, 0x70, 0x4b, 0x4f, 0x50, 0x59, 0x51, 0x65, 0x73, 0x66, 0x5a,
    0x59, 0x33, 0x66, 0x77, 0x74, 0x32, 0x33, 0x74, 0x77, 0x22, 0x2c, 0x22, 0x6d, 0x61, 0x78, 0x5f,
    0x62, 0x61, 0x74, 0x63, 0x68, 0x5f, 0x71, 0x75, 0x65, 0x72, 0x79, 0x5f, 0x63, 0x6f, 0x75, 0x6e,
    0x74, 0x22, 0x3a, 0x31, 0x2c, 0x22, 0x74, 0x61, 0x73, 0x6b, 0x5f, 0x65, 0x78, 0x70, 0x69, 0x72,
    0x61, 0x74, 0x69, 0x6f, 0x6e, 0x22, 0x3a, 0x31, 0x37, 0x33, 0x32, 0x30, 0x37, 0x30, 0x36, 0x35,
    0x31, 0x2c, 0x22, 0x72, 0x65, 0x70, 0x6f, 0x72, 0x74, 0x5f, 0x65, 0x78, 0x70, 0x69, 0x72, 0x79,
    0x5f, 0x61, 0x67, 0x65, 0x22, 0x3a, 0x31, 0x32, 0x30, 0x39, 0x36, 0x30, 0x30, 0x2c, 0x22, 0x6d,
    0x69, 0x6e, 0x5f, 0x62, 0x61, 0x74, 0x63, 0x68, 0x5f, 0x73, 0x69, 0x7a, 0x65, 0x22, 0x3a, 0x31,
    0x30, 0x30, 0x2c, 0x22, 0x74, 0x69, 0x6d, 0x65, 0x5f, 0x70, 0x72, 0x65, 0x63, 0x69, 0x73, 0x69,
    0x6f, 0x6e, 0x22, 0x3a, 0x32, 0x38, 0x38, 0x30, 0x30, 0x2c, 0x22, 0x74, 0x6f, 0x6c, 0x65, 0x72,
    0x61, 0x62, 0x6c, 0x65, 0x5f, 0x63, 0x6c, 0x6f, 0x63, 0x6b, 0x5f, 0x73, 0x6b, 0x65, 0x77, 0x22,
    0x3a, 0x36, 0x30, 0x2c, 0x22, 0x61, 0x67, 0x67, 0x72, 0x65, 0x67, 0x61, 0x74, 0x6f, 0x72, 0x5f,
    0x61, 0x75, 0x74, 0x68, 0x5f, 0x74, 0x6f, 0x6b, 0x65, 0x6e, 0x22, 0x3a, 0x7b, 0x22, 0x74, 0x79,
    0x70, 0x65, 0x22, 0x3a, 0x22, 0x42, 0x65, 0x61, 0x72, 0x65, 0x72, 0x22, 0x2c, 0x22, 0x74, 0x6f,
    0x6b, 0x65, 0x6e, 0x22, 0x3a, 0x22, 0x42, 0x54, 0x70, 0x6e, 0x56, 0x65, 0x66, 0x6a, 0x6e, 0x56,
    0x45, 0x46, 0x75, 0x72, 0x6c, 0x4a, 0x5f, 0x66, 0x51, 0x55, 0x59, 0x51, 0x22, 0x7d, 0x2c, 0x22,
    0x63, 0x6f, 0x6c, 0x6c, 0x65, 0x63, 0x74, 0x6f, 0x72, 0x5f, 0x68, 0x70, 0x6b, 0x65, 0x5f, 0x63,
    0x6f, 0x6e, 0x66, 0x69, 0x67, 0x22, 0x3a, 0x7b, 0x22, 0x69, 0x64, 0x22, 0x3a, 0x36, 0x38, 0x2c,
    0x22, 0x6b, 0x65, 0x6d, 0x5f, 0x69, 0x64, 0x22, 0x3a, 0x22, 0x58, 0x32, 0x35, 0x35, 0x31, 0x39,
    0x48, 0x6b, 0x64, 0x66, 0x53, 0x68, 0x61, 0x32, 0x35, 0x36, 0x22, 0x2c, 0x22, 0x6b, 0x64, 0x66,
    0x5f, 0x69, 0x64, 0x22, 0x3a, 0x22, 0x48, 0x6b, 0x64, 0x66, 0x53, 0x68, 0x61, 0x32, 0x35, 0x36,
    0x22, 0x2c, 0x22, 0x61, 0x65, 0x61, 0x64, 0x5f, 0x69, 0x64, 0x22, 0x3a, 0x22, 0x41, 0x65, 0x73,
    0x31, 0x32, 0x38, 0x47, 0x63, 0x6d, 0x22, 0x2c, 0x22, 0x70, 0x75, 0x62, 0x6c, 0x69, 0x63, 0x5f,
    0x6b, 0x65, 0x79, 0x22, 0x3a, 0x22, 0x66, 0x5f, 0x73, 0x49, 0x64, 0x36, 0x54, 0x6d, 0x59, 0x75,
    0x33, 0x48, 0x4b, 0x47, 0x47, 0x2d, 0x4b, 0x42, 0x53, 0x79, 0x67, 0x50, 0x50, 0x2d, 0x5a, 0x33,
    0x73, 0x62, 0x38, 0x57, 0x73, 0x65, 0x75, 0x65, 0x57, 0x64, 0x55, 0x77, 0x4c, 0x43, 0x49, 0x6c,
    0x4d, 0x22, 0x7d, 0x2c, 0x22, 0x61, 0x67, 0x67, 0x72, 0x65, 0x67, 0x61, 0x74, 0x6f, 0x72, 0x5f,
    0x68, 0x70, 0x6b, 0x65, 0x5f, 0x63, 0x6f, 0x6e, 0x66, 0x69, 0x67, 0x73, 0x22, 0x3a, 0x5b, 0x7b,
    0x22, 0x69, 0x64, 0x22, 0x3a, 0x31, 0x30, 0x38, 0x2c, 0x22, 0x6b, 0x65, 0x6d, 0x5f, 0x69, 0x64,
    0x22, 0x3a, 0x22, 0x58, 0x32, 0x35, 0x35, 0x31, 0x39, 0x48, 0x6b, 0x64, 0x66, 0x53, 0x68, 0x61,
    0x32, 0x35, 0x36, 0x22, 0x2c, 0x22, 0x6b, 0x64, 0x66, 0x5f, 0x69, 0x64, 0x22, 0x3a, 0x22, 0x48,
    0x6b, 0x64, 0x66, 0x53, 0x68, 0x61, 0x32, 0x35, 0x36, 0x22, 0x2c, 0x22, 0x61, 0x65, 0x61, 0x64,
    0x5f, 0x69, 0x64, 0x22, 0x3a, 0x22, 0x41, 0x65, 0x73, 0x31, 0x32, 0x38, 0x47, 0x63, 0x6d, 0x22,
    0x2c, 0x22, 0x70, 0x75, 0x62, 0x6c, 0x69, 0x63, 0x5f, 0x6b, 0x65, 0x79, 0x22, 0x3a, 0x22, 0x38,
    0x44, 0x4f, 0x59, 0x7a, 0x49, 0x6f, 0x61, 0x58, 0x76, 0x70, 0x54, 0x34, 0x69, 0x66, 0x4f, 0x7a,
    0x6f, 0x68, 0x48, 0x53, 0x48, 0x64, 0x45, 0x75, 0x42, 0x56, 0x61, 0x2d, 0x7a, 0x64, 0x6c, 0x5f,
    0x6b, 0x32, 0x67, 0x6a, 0x76, 0x31, 0x6b, 0x47, 0x6c, 0x63, 0x22, 0x7d, 0x5d, 0x7d,
];

pub struct CannedConnector {
    recv: Vec<u8>,
}

impl CannedConnector {
    pub fn new(recv_data: Vec<u8>) -> Self {
        Self { recv: recv_data }
    }
}

#[async_trait]
impl Connector for CannedConnector {
    type Transport = CursorTransport;

    async fn connect(&self, _url: &Url) -> io::Result<Self::Transport> {
        Ok(CursorTransport::new(self.recv.clone()))
    }

    fn spawn<Fut: Future<Output = ()> + Send + 'static>(&self, fut: Fut) {
        spawn(fut)
    }
}

pub struct CursorTransport {
    send: Cursor<Vec<u8>>,
    recv: Cursor<Vec<u8>>,
}

impl Unpin for CursorTransport {}

impl CursorTransport {
    pub fn new(recv_data: Vec<u8>) -> Self {
        Self {
            send: Cursor::new(Vec::new()),
            recv: Cursor::new(recv_data),
        }
    }
}

impl Transport for CursorTransport {}

impl AsyncRead for CursorTransport {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let out = Poll::Ready(self.as_mut().recv.read(buf));
        trace!("Receiving data: {:?}", String::from_utf8_lossy(&buf));
        out
    }
}

impl AsyncWrite for CursorTransport {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        trace!("Sending data: {:?}", String::from_utf8_lossy(buf));
        Poll::Ready(self.as_mut().send.write(buf))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use futures_lite::future::block_on;
    use trillium::KnownHeaderName;
    use trillium_client::Client;
    use url::Url;

    use crate::{CannedConnector, CANNED_RESPONSE_BODY, REQUEST_BODY};

    #[test]
    fn caddy_repro() {
        let _ = env_logger::builder().is_test(true).try_init();

        let connector = CannedConnector::new(CANNED_RESPONSE_BODY.to_vec());
        let client = Client::new(connector).with_default_pool();
        let url =
            Url::parse("http://aggregator.kind-ci-castor.svc.cluster.local/aggregator-api/tasks")
                .unwrap();
        block_on(async move {
            let conn = client
                .post(url.clone())
                .with_header(
                    KnownHeaderName::Accept,
                    "application/vnd.janus.aggregator+json;version=0.1",
                )
                .with_header(
                    KnownHeaderName::ContentType,
                    "application/vnd.janus.aggregator+json;version=0.1",
                )
                .with_header(
                    KnownHeaderName::Authorization,
                    "Bearer Oe-WIwqEHQmSMGSbxYoMBuiIvHPIYO6l",
                )
                .with_body(REQUEST_BODY.as_slice());
            let mut conn = conn.await.unwrap();
            dbg!(conn.status());
            let body = conn.response_body();
            dbg!(&body);
            dbg!(body.await.unwrap());
        });
    }
}
