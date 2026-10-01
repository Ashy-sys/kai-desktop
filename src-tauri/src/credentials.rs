use std::{ptr, slice};
use windows_sys::Win32::Foundation::{GetLastError, ERROR_NOT_FOUND};
use windows_sys::Win32::Security::Credentials::{
    CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
    CRED_TYPE_GENERIC,
};
use zeroize::Zeroize;

pub const OPENAI_CREDENTIAL_TARGET: &str = "Kai Desktop / OpenAI API";

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

pub fn validate_api_key(value: &str) -> Result<(), String> {
    if value.len() < 20
        || value.len() > 512
        || !value.starts_with("sk-")
        || value.chars().any(char::is_control)
    {
        return Err("That doesn't look like a valid OpenAI API key.".into());
    }
    Ok(())
}

pub fn write_openai_key(mut value: String) -> Result<(), String> {
    validate_api_key(&value)?;
    let mut target = wide(OPENAI_CREDENTIAL_TARGET);
    let mut username = wide("OpenAI API key");
    let bytes = value.as_bytes();
    let mut credential: CREDENTIALW = unsafe { std::mem::zeroed() };
    credential.Type = CRED_TYPE_GENERIC;
    credential.TargetName = target.as_mut_ptr();
    credential.UserName = username.as_mut_ptr();
    credential.CredentialBlobSize = bytes.len() as u32;
    credential.CredentialBlob = bytes.as_ptr() as *mut u8;
    credential.Persist = CRED_PERSIST_LOCAL_MACHINE;
    let result = unsafe { CredWriteW(&credential, 0) };
    value.zeroize();
    if result == 0 {
        Err("Windows Credential Manager couldn't save the API key.".into())
    } else {
        Ok(())
    }
}

pub fn read_openai_key() -> Result<Option<String>, String> {
    let target = wide(OPENAI_CREDENTIAL_TARGET);
    let mut raw: *mut CREDENTIALW = ptr::null_mut();
    let result = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut raw) };
    if result == 0 {
        return if unsafe { GetLastError() } == ERROR_NOT_FOUND {
            Ok(None)
        } else {
            Err("Windows Credential Manager couldn't read the API-key status.".into())
        };
    }
    if raw.is_null() {
        return Err("Windows Credential Manager returned an invalid credential.".into());
    }
    let credential = unsafe { &*raw };
    let bytes = unsafe {
        slice::from_raw_parts(
            credential.CredentialBlob,
            credential.CredentialBlobSize as usize,
        )
    };
    let value =
        String::from_utf8(bytes.to_vec()).map_err(|_| "The saved API key is invalid.".to_string());
    unsafe { CredFree(raw as *const _) };
    value.map(Some)
}

pub fn delete_openai_key() -> Result<(), String> {
    let target = wide(OPENAI_CREDENTIAL_TARGET);
    let result = unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) };
    if result != 0 || unsafe { GetLastError() } == ERROR_NOT_FOUND {
        Ok(())
    } else {
        Err("Windows Credential Manager couldn't remove the API key.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_key_shape_without_exposing_it() {
        let shaped_key = ["sk", "test", "abcdefghijklmnopqrstuvwxyz"].join("-");
        assert!(validate_api_key(&shaped_key).is_ok());
        assert!(validate_api_key("not-a-key").is_err());
        assert_eq!(OPENAI_CREDENTIAL_TARGET, "Kai Desktop / OpenAI API");
    }
}
