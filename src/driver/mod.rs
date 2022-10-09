// Copyright 2020-2022 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! ***NitroSecureModule driver communication support***
//! # Overview
//! This module implements support functions for communicating with the NSM
//! driver by encoding requests to / decoding responses from a C-compatible
//! message structure which is shared with the driver via `ioctl()`.
//! In general, a message contains:
//! 1. A *request* content structure, holding CBOR-encoded user input data.
//! 2. A *response* content structure, with an initial memory capacity provided by
//! the user, which then gets populated with information from the NSM driver and
//! then decoded from CBOR.

use crate::api::{Request, Response};
use libc::ioctl;
use nix::request_code_readwrite;
use nix::unistd::close;
use std::io::{IoSlice, IoSliceMut};

use std::fs::OpenOptions;
use std::mem;
use std::os::unix::io::{IntoRawFd, RawFd};

const DEV_FILE: &str = "/dev/nsm";
const NSM_IOCTL_MAGIC: u8 = 0x0A;
const NSM_RESPONSE_MAX_SIZE: usize = 0x3000;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Cbor(serde_cbor::Error),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<serde_cbor::Error> for Error {
    fn from(err: serde_cbor::Error) -> Self {
        Error::Cbor(err)
    }
}

type Result<T> = std::result::Result<T, Error>;

/// NSM message structure to be used with `ioctl()`.
#[repr(C)]
struct NsmMessage<'a> {
    /// User-provided data for the request
    pub request: IoSlice<'a>,
    /// Response data provided by the NSM pipeline
    pub response: IoSliceMut<'a>,
}

/// Encode an NSM `Request` value into a vector.  
/// *Argument 1 (input)*: The NSM request.  
/// *Returns*: The vector containing the CBOR encoding.
fn nsm_encode_request_to_cbor(request: Request) -> Result<Vec<u8>> {
    serde_cbor::to_vec(&request).map_err(|err| Error::Cbor(err))
}

/// Decode an NSM `Response` value from a raw memory buffer.  
/// *Argument 1 (input)*: The `iovec` holding the memory buffer.  
/// *Returns*: The decoded NSM response.
fn nsm_decode_response_from_cbor(response_data: &[u8]) -> Result<Response> {
    serde_cbor::from_slice(response_data).map_err(|err| Error::Cbor(err))
}

/// Do an `ioctl()` of a given type for a given message.  
/// *Argument 1 (input)*: The descriptor to the device file.  
/// *Argument 2 (input/output)*: The message to be sent and updated via `ioctl()`.  
/// *Returns*: The status of the operation.
fn nsm_ioctl(fd: i32, message: &mut NsmMessage) -> Result<()> {
    let status = unsafe {
        ioctl(
            fd,
            request_code_readwrite!(NSM_IOCTL_MAGIC, 0, mem::size_of::<NsmMessage>()),
            message,
        )
    };

    match status {
        // If ioctl() succeeded, the status is the message's response code
        0 => Ok(()),

        // If ioctl() failed, the error is given by errno
        _ => Err(std::io::Error::last_os_error().into()),
    }
}

/// Create a message with input data and output capacity from a given
/// request, then send it to the NSM driver via `ioctl()` and wait
/// for the driver's response.  
/// *Argument 1 (input)*: The descriptor to the NSM device file.  
/// *Argument 2 (input)*: The NSM request.  
/// *Returns*: The corresponding NSM response from the driver.
pub fn nsm_process_request(fd: i32, request: Request) -> Result<Response> {
    let cbor_request = nsm_encode_request_to_cbor(request)?;

    let mut cbor_response: [u8; NSM_RESPONSE_MAX_SIZE] = [0; NSM_RESPONSE_MAX_SIZE];
    let mut message = NsmMessage {
        request: IoSlice::new(&cbor_request),
        response: IoSliceMut::new(&mut cbor_response),
    };
    let _ = nsm_ioctl(fd, &mut message)?;

    Ok(nsm_decode_response_from_cbor(&message.response)?)
}

/// NSM library initialization function.  
/// *Returns*: A descriptor for the opened device file.
pub fn nsm_init() -> Result<RawFd> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(DEV_FILE)
        .map(|file| file.into_raw_fd())
        .map_err(|err| Error::Io(err))
}

/// NSM library exit function.  
/// *Argument 1 (input)*: The descriptor for the opened device file, as
/// obtained from `nsm_init()`.
pub fn nsm_exit(fd: RawFd) -> Result<()> {
    match close(fd as RawFd) {
        Err(err) => Err(Error::Io(err.into())),
        _ => Ok(()),
    }
}
