// Copyright 2022 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! AWS Nitro Secure Module API
//!
//! This is the library that provides the API for the Nitro Secure Module used in AWS Nitro
//! Enclaves for management, attestation and entropy generation.
//!
//! nsm_io provides the API and CBOR encoding functionality.
//! nsm_driver provides the ioctl interface for the Nitro Secure Module driver.

pub mod api;
pub mod driver;

use std::os::unix::io::RawFd;

use api::Request;
use driver::{nsm_exit, nsm_init, nsm_process_request};

pub struct NitroSecureModule {
    fd: RawFd,
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Cbor(serde_cbor::Error),
    NitroSecureModuleError(api::ErrorCode),
    InvalidReponse,
}

impl From<driver::Error> for Error {
    fn from(err: driver::Error) -> Self {
        match err {
            driver::Error::Io(io_err) => Error::Io(io_err),
            driver::Error::Cbor(cbor_err) => Error::Cbor(cbor_err),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

impl NitroSecureModule {
    pub fn new() -> Result<Self> {
        let fd = nsm_init()?;
        Ok(Self { fd })
    }

    pub fn get_random(&self) -> Result<Vec<u8>> {
        let request = Request::GetRandom;
        let response = nsm_process_request(self.fd, request)?;
        match response {
            api::Response::GetRandom { random } => Ok(random),
            api::Response::Error(err_code) => Err(Error::NitroSecureModuleError(err_code)),
            _ => Err(Error::InvalidReponse),
        }
    }
}

impl Drop for NitroSecureModule {
    fn drop(&mut self) {
        // Purposefully ignore errors since only other option is log or panic.
        nsm_exit(self.fd).unwrap_or_default()
    }
}
