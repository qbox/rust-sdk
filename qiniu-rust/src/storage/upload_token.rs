use super::upload_policy::UploadPolicy;
use crate::{credential::Credential, utils::base64};
use error_chain::error_chain;
use std::{borrow::Cow, convert::From, fmt};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UploadToken<'p> {
    // TODO: Token 缓存 Policy，Policy 缓存 Token
    Token(Cow<'p, str>),
    Policy(UploadPolicy<'p>, Cow<'p, Credential>),
}

impl<'p> UploadToken<'p> {
    pub fn from_token<T: Into<Cow<'p, str>>>(t: T) -> UploadToken<'p> {
        UploadToken::Token(t.into())
    }

    pub fn from_policy(policy: UploadPolicy<'p>, credential: Cow<'p, Credential>) -> UploadToken<'p> {
        UploadToken::Policy(policy, credential)
    }

    pub fn access_key(&self) -> Result<&str> {
        match self {
            UploadToken::Token(token) => token
                .find(':')
                .map(|i| token.split_at(i).0)
                .ok_or_else(|| ErrorKind::InvalidUploadTokenFormat.into()),
            UploadToken::Policy(_, credential) => Ok(credential.access_key()),
        }
    }

    pub fn policy<'a>(&'a self) -> Result<Cow<'a, UploadPolicy<'p>>> {
        match self {
            UploadToken::Token(token) => {
                let encoded_policy = token.splitn(3, ':').last().ok_or(ErrorKind::InvalidUploadTokenFormat)?;
                let decoded_policy = base64::decode(encoded_policy.as_bytes()).map_err(ErrorKind::Base64DecodeError)?;
                Ok(Cow::Owned(
                    UploadPolicy::from_json_slice_owned(&decoded_policy).map_err(ErrorKind::JSONDecodeError)?,
                ))
            }
            UploadToken::Policy(policy, _) => Ok(Cow::Borrowed(policy)),
        }
    }

    pub fn token(self) -> String {
        match self {
            UploadToken::Token(token) => token.into_owned(),
            UploadToken::Policy(policy, credential) => credential.sign_upload_policy(&policy),
        }
    }
}

impl fmt::Display for UploadToken<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            UploadToken::Token(token) => fmt::Display::fmt(token, f),
            UploadToken::Policy(policy, credential) => fmt::Display::fmt(&credential.sign_upload_policy(policy), f),
        }
    }
}

impl From<String> for UploadToken<'_> {
    fn from(s: String) -> Self {
        Self::from_token(s)
    }
}

impl<'p> From<&'p str> for UploadToken<'p> {
    fn from(s: &'p str) -> Self {
        Self::from_token(s)
    }
}

impl<'p> From<UploadToken<'p>> for String {
    fn from(upload_token: UploadToken<'p>) -> Self {
        upload_token.token()
    }
}

error_chain! {
    errors {
        InvalidUploadTokenFormat {
            description("Invalid upload token format")
            display("Invalid upload token format")
        }
        Base64DecodeError(err: base64::DecodeError) {
            description("Base64 decode error")
            display("Base64 decode error: {}", err)
        }
        JSONDecodeError(err: serde_json::Error) {
            description("JSON decode error")
            display("JSON decode error: {}", err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{super::upload_policy::UploadPolicyBuilder, *};
    use crate::config::Config;
    use std::{borrow::Cow, boxed::Box, error::Error, result::Result};

    #[test]
    fn test_build_upload_token_from_upload_policy() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_object("test_bucket", "test:file", &Config::default()).build();
        let token = UploadToken::from_policy(policy, Cow::Owned(get_credential())).token();
        assert!(token.starts_with(get_credential().access_key()));
        let token = UploadToken::from_token(token);
        let policy = token.policy()?;
        assert_eq!(policy.bucket(), Some("test_bucket"));
        assert_eq!(policy.key(), Some("test:file"));
        accept_string(token.to_owned().into());
        accept_upload_token(&token.to_string().into());
        accept_upload_token(&token.to_string().as_str().into());
        Ok(())
    }

    fn accept_string(_: String) {}
    fn accept_upload_token(_: &UploadToken) {}

    fn get_credential() -> Credential {
        Credential::new("abcdefghklmnopq", "1234567890")
    }
}
