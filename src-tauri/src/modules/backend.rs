use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use image::{codecs::jpeg::JpegEncoder, DynamicImage};
use rusqlite::{params, types::Value, Connection};
use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, State, WebviewWindow, Window,
    WindowEvent,
};
use thiserror::Error;
use uuid::Uuid;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

#[path = "../utils.rs"]
mod utils;
use utils::{display_path, non_empty_name, optional_text};

const SETTINGS_FILE: &str = "settings.json";
const DATABASE_FILE: &str = "library.db";
const LOG_DIRECTORY: &str = "logs";
const LOG_FILE: &str = "asset-library.log";

#[derive(Debug, Error)]
enum LibraryError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

type LibraryResult<T> = Result<T, LibraryError>;

mod state;
use state::*;
mod models;
use models::*;
mod commands;
use commands::handler;
mod database;
use database::*;
mod thumbnails;
use thumbnails::*;
mod storage;
use storage::*;
mod imports;
use imports::*;
mod schema;
use schema::*;
mod runtime;
pub use runtime::run;
#[cfg(test)]
mod tests;
