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

use driver::{nsm_exit, nsm_init};

pub struct NitroSecureModule {
    fd: RawFd
}

impl NitroSecureModule {
    pub fn new() -> std::io::Result<Self> {
        let fd = nsm_init()?;
        Ok(Self {
            fd
        })
    }
}

impl Drop for NitroSecureModule {
    fn drop(&mut self) {
        // Purposefully ignore errors since only other option is log or panic.
        nsm_exit(self.fd).unwrap_or_default()
    }
}
