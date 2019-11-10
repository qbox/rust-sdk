use super::{
    super::{upload_policy::UploadPolicy, upload_token::UploadToken},
    form_uploader::FormUploaderBuilder,
    resumeable_uploader::{ResumeableUploader, ResumeableUploaderBuilder},
    upload_recorder::UploadRecorder,
    UploadLoggerBuilder, UploadResponse,
};
use crate::{config::Config, credential::Credential, http::Client, utils::ron::Ron};
use assert_impl::assert_impl;
use getset::Getters;
use mime::Mime;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::{
    borrow::Cow,
    collections::HashMap,
    fs::File,
    io::{Error as IOError, Read, Result as IOResult},
    path::Path,
    sync::Arc,
};
use thiserror::Error;

#[derive(Getters)]
#[get = "pub(super)"]
pub struct BucketUploaderInner {
    bucket_name: Box<str>,
    up_urls_list: Box<[Box<[Box<str>]>]>,
    http_client: Client,
    upload_logger_builder: Option<UploadLoggerBuilder>,
    recorder: UploadRecorder,
    thread_pool: Option<ThreadPool>,
}

#[derive(Clone)]
pub struct BucketUploader {
    inner: Arc<BucketUploaderInner>,
}

impl BucketUploader {
    pub(super) fn bucket_name(&self) -> &str {
        self.inner.bucket_name()
    }
    pub(super) fn up_urls_list(&self) -> &[Box<[Box<str>]>] {
        self.inner.up_urls_list()
    }
    pub(super) fn http_client(&self) -> &Client {
        self.inner.http_client()
    }
    pub(super) fn upload_logger_builder(&self) -> Option<&UploadLoggerBuilder> {
        self.inner.upload_logger_builder().as_ref()
    }
    pub(super) fn recorder(&self) -> &UploadRecorder {
        self.inner.recorder()
    }
    pub(super) fn thread_pool(&self) -> Option<&ThreadPool> {
        self.inner.thread_pool().as_ref()
    }
}

pub struct BucketUploaderBuilder {
    inner: BucketUploaderInner,
}

impl BucketUploaderBuilder {
    pub(super) fn new(
        bucket_name: Box<str>,
        up_urls_list: Box<[Box<[Box<str>]>]>,
        config: Config,
        upload_logger_builder: Option<UploadLoggerBuilder>,
    ) -> BucketUploaderBuilder {
        assert!(!up_urls_list.is_empty());
        BucketUploaderBuilder {
            inner: BucketUploaderInner {
                bucket_name,
                up_urls_list,
                recorder: UploadRecorder::new(config.recorder().to_owned(), &config),
                http_client: Client::new(config),
                thread_pool: None,
                upload_logger_builder,
            },
        }
    }

    pub fn thread_pool_size(mut self, num_threads: usize) -> BucketUploaderBuilder {
        self.inner.thread_pool = Some(
            ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .thread_name(move |index| format!("bucket_uploader_thread_{}_{}", num_threads, index))
                .build()
                .unwrap(),
        );
        self
    }

    pub fn build(self) -> BucketUploader {
        BucketUploader {
            inner: Arc::new(self.inner),
        }
    }
}

impl BucketUploader {
    pub fn upload_token<'b>(&'b self, upload_token: impl Into<UploadToken<'b>>) -> FileUploaderBuilder<'b> {
        FileUploaderBuilder::new(Ron::Referenced(self), upload_token.into().token().into())
    }

    pub fn upload_policy<'b>(
        &'b self,
        upload_policy: UploadPolicy<'b>,
        credential: Cow<'b, Credential>,
    ) -> FileUploaderBuilder<'b> {
        FileUploaderBuilder::new(
            Ron::Referenced(self),
            UploadToken::from_policy(upload_policy, credential).token().into(),
        )
    }

    pub unsafe fn from_raw(ptr: *const BucketUploaderInner) -> BucketUploader {
        BucketUploader {
            inner: Arc::from_raw(ptr),
        }
    }

    pub fn into_raw(self) -> *const BucketUploaderInner {
        Arc::into_raw(self.inner)
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

pub enum ResumeablePolicy {
    Threshold(u64),
    Never,
    Always,
}

pub struct FileUploaderBuilder<'b> {
    bucket_uploader: Ron<'b, BucketUploader>,
    upload_token: Cow<'b, str>,
    key: Option<Cow<'b, str>>,
    vars: Option<HashMap<Cow<'b, str>, Cow<'b, str>>>,
    metadata: Option<HashMap<Cow<'b, str>, Cow<'b, str>>>,
    checksum_enabled: bool,
    resumeable_policy: ResumeablePolicy,
    on_uploading_progress: Option<&'b (dyn Fn(usize, Option<usize>) + Send + Sync)>,
    thread_pool: Option<Ron<'b, ThreadPool>>,
}

impl<'b> FileUploaderBuilder<'b> {
    pub(super) fn new(bucket_uploader: Ron<'b, BucketUploader>, upload_token: Cow<'b, str>) -> FileUploaderBuilder<'b> {
        FileUploaderBuilder {
            upload_token,
            key: None,
            vars: None,
            metadata: None,
            checksum_enabled: true,
            on_uploading_progress: None,
            thread_pool: None,
            resumeable_policy: ResumeablePolicy::Threshold(bucket_uploader.http_client().config().upload_threshold()),
            bucket_uploader,
        }
    }

    pub fn thread_pool_ref(mut self, thread_pool: &'b ThreadPool) -> FileUploaderBuilder<'b> {
        self.thread_pool = Some(Ron::Referenced(thread_pool));
        self
    }

    pub fn thread_pool(mut self, thread_pool: ThreadPool) -> FileUploaderBuilder<'b> {
        self.thread_pool = Some(Ron::Owned(thread_pool));
        self
    }

    pub fn key(mut self, key: impl Into<Cow<'b, str>>) -> FileUploaderBuilder<'b> {
        self.key = Some(key.into());
        self
    }

    pub fn var(mut self, key: impl Into<Cow<'b, str>>, value: impl Into<Cow<'b, str>>) -> FileUploaderBuilder<'b> {
        if let Some(vars) = &mut self.vars {
            vars.insert(key.into(), value.into());
        } else {
            let mut vars = HashMap::with_capacity(1);
            vars.insert(key.into(), value.into());
            self.vars = Some(vars);
        }
        self
    }

    pub fn metadata(mut self, key: impl Into<Cow<'b, str>>, value: impl Into<Cow<'b, str>>) -> FileUploaderBuilder<'b> {
        if let Some(metadata) = &mut self.metadata {
            metadata.insert(key.into(), value.into());
        } else {
            let mut metadata = HashMap::with_capacity(1);
            metadata.insert(key.into(), value.into());
            self.metadata = Some(metadata);
        }
        self
    }

    pub fn disable_checksum(mut self) -> FileUploaderBuilder<'b> {
        self.checksum_enabled = false;
        self
    }

    pub fn enable_checksum(mut self) -> FileUploaderBuilder<'b> {
        self.checksum_enabled = true;
        self
    }

    pub fn upload_threshold(mut self, threshold: u64) -> FileUploaderBuilder<'b> {
        self.resumeable_policy = ResumeablePolicy::Threshold(threshold);
        self
    }

    pub fn always_be_resumeable(mut self) -> FileUploaderBuilder<'b> {
        self.resumeable_policy = ResumeablePolicy::Always;
        self
    }

    pub fn never_be_resumeable(mut self) -> FileUploaderBuilder<'b> {
        self.resumeable_policy = ResumeablePolicy::Never;
        self
    }

    pub fn on_progress(
        mut self,
        callback: &'b (dyn Fn(usize, Option<usize>) + Send + Sync),
    ) -> FileUploaderBuilder<'b> {
        self.on_uploading_progress = Some(callback);
        self
    }

    pub fn upload_file<'n>(
        self,
        file_path: impl AsRef<Path>,
        file_name: Option<impl Into<Cow<'n, str>>>,
        mime: Option<Mime>,
    ) -> UploadResult {
        let file_path = file_path.as_ref();
        let file_name = file_name.map(|file_name| file_name.into());
        match self.resumeable_policy {
            ResumeablePolicy::Threshold(threshold) => {
                if file_path.metadata()?.len() > threshold {
                    self.upload_file_by_blocks(file_path, file_name, mime)
                } else {
                    self.upload_file_by_form(file_path, file_name, mime)
                }
            }
            ResumeablePolicy::Always => self.upload_file_by_blocks(file_path, file_name, mime),
            ResumeablePolicy::Never => self.upload_file_by_form(file_path, file_name, mime),
        }
    }

    pub fn upload_stream<'n>(
        self,
        stream: impl Read + Send + Sync,
        file_name: Option<impl Into<Cow<'n, str>>>,
        mime: Option<Mime>,
    ) -> UploadResult {
        let file_name = file_name.map(|file_name| file_name.into());
        match self.resumeable_policy {
            ResumeablePolicy::Threshold(_) | ResumeablePolicy::Always => {
                self.upload_stream_by_blocks(stream, file_name, mime)
            }
            ResumeablePolicy::Never => self.upload_stream_by_form(stream, file_name, mime),
        }
    }

    fn upload_file_by_form<'n>(
        self,
        file_path: &Path,
        file_name: Option<Cow<'n, str>>,
        mime: Option<Mime>,
    ) -> UploadResult {
        let mut uploader = FormUploaderBuilder::new(&self.bucket_uploader, &self.upload_token);
        if let Some(key) = self.key {
            uploader = uploader.key(key);
        }
        if let Some(vars) = self.vars {
            for (k, v) in vars.into_iter() {
                uploader = uploader.var(&k, v);
            }
        }
        if let Some(metadata) = self.metadata {
            for (k, v) in metadata.into_iter() {
                uploader = uploader.metadata(&k, v);
            }
        }
        if let Some(callback) = self.on_uploading_progress {
            uploader = uploader.on_uploading_progress(callback);
        }
        Ok(uploader
            .seekable_stream(
                File::open(file_path)?,
                Self::guess_filename(file_path, file_name),
                Self::guess_mime_from_file_path(mime, file_path),
                self.checksum_enabled,
            )?
            .send()?)
    }

    fn upload_file_by_blocks<'n>(
        self,
        file_path: &Path,
        file_name: Option<Cow<'n, str>>,
        mime: Option<Mime>,
    ) -> UploadResult {
        let mut uploader = ResumeableUploaderBuilder::new(&self.bucket_uploader, self.upload_token);
        if let Some(key) = &self.key {
            uploader = uploader.key(key.to_owned());
        }
        if let Some(vars) = self.vars {
            uploader = uploader.vars(vars);
        }
        if let Some(metadata) = self.metadata {
            uploader = uploader.metadata(metadata);
        }
        if let Some(callback) = self.on_uploading_progress {
            uploader = uploader.on_uploading_progress(callback);
        }
        if let Some(thread_pool_or_referenced) = self.thread_pool {
            uploader = uploader.thread_pool_or_referenced(thread_pool_or_referenced);
        }
        let mut uploader = uploader.file(
            File::open(file_path)?,
            file_path.into(),
            Self::guess_filename(file_path, file_name),
            file_path.metadata()?.len(),
            Self::guess_mime_from_file_path(mime, file_path),
            self.checksum_enabled,
        )?;
        Self::prepare_for_resuming(
            self.key.as_ref().map(|key| key.as_ref()),
            &self.bucket_uploader.recorder(),
            &mut uploader,
            file_path,
        )?;
        Ok(uploader.send()?)
    }

    fn prepare_for_resuming(
        key: Option<&str>,
        recorder: &UploadRecorder,
        uploader: &mut ResumeableUploader<'_, File>,
        file_path: &Path,
    ) -> IOResult<()> {
        if let Some((file_record, block_records)) = recorder.load(file_path, key)? {
            uploader.prepare_for_resuming(file_record, block_records, recorder.open_for_appending(file_path, key)?);
        }
        Ok(())
    }

    fn upload_stream_by_form<R: Read>(
        self,
        stream: R,
        file_name: Option<Cow<str>>,
        mime: Option<Mime>,
    ) -> UploadResult {
        let mut uploader = FormUploaderBuilder::new(&self.bucket_uploader, &self.upload_token);
        if let Some(key) = self.key {
            uploader = uploader.key(key);
        }
        if let Some(vars) = self.vars {
            for (k, v) in vars.into_iter() {
                uploader = uploader.var(&k, v);
            }
        }
        if let Some(metadata) = self.metadata {
            for (k, v) in metadata.into_iter() {
                uploader = uploader.metadata(&k, v);
            }
        }
        if let Some(callback) = self.on_uploading_progress {
            uploader = uploader.on_uploading_progress(callback);
        }
        Ok(uploader
            .stream(
                stream,
                Self::guess_mime_from_file_name(mime, file_name.as_ref().map(|name| name.as_ref())),
                file_name,
                None,
            )?
            .send()?)
    }

    fn upload_stream_by_blocks<R: Read + Send + Sync>(
        self,
        stream: R,
        file_name: Option<Cow<str>>,
        mime: Option<Mime>,
    ) -> UploadResult {
        let mut uploader = ResumeableUploaderBuilder::new(&self.bucket_uploader, self.upload_token);
        if let Some(key) = self.key {
            uploader = uploader.key(key);
        }
        if let Some(vars) = self.vars {
            uploader = uploader.vars(vars);
        }
        if let Some(metadata) = self.metadata {
            uploader = uploader.metadata(metadata);
        }
        if let Some(callback) = self.on_uploading_progress {
            uploader = uploader.on_uploading_progress(callback);
        }
        if let Some(thread_pool_or_referenced) = self.thread_pool {
            uploader = uploader.thread_pool_or_referenced(thread_pool_or_referenced);
        }
        Ok(uploader
            .stream(
                stream,
                Self::guess_mime_from_file_name(mime, file_name.as_ref().map(|name| name.as_ref())),
                file_name,
                true,
            )?
            .send()?)
    }

    fn guess_filename<'n>(file_path: &Path, file_name: Option<Cow<'n, str>>) -> Option<Cow<'n, str>> {
        file_name.or_else(|| {
            file_path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_owned().into())
        })
    }

    fn guess_mime_from_file_path(mime: Option<Mime>, file_path: &Path) -> Option<Mime> {
        mime.or_else(|| mime_guess::from_path(file_path).first())
    }

    fn guess_mime_from_file_name(mime: Option<Mime>, file_name: Option<&str>) -> Option<Mime> {
        mime.or_else(|| file_name.and_then(|file_name| mime_guess::from_path(file_name).first()))
    }
}

#[derive(Error, Debug)]
pub enum UploadError {
    #[error("Failed to do local io operation during uploading: {0}")]
    IOError(#[from] IOError),
    #[error("Qiniu API call error: {0}")]
    QiniuError(#[from] qiniu_http::Error),
}
pub type UploadResult = Result<UploadResponse, UploadError>;
