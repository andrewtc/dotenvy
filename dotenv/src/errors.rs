use std::env;
use std::error;
use std::fmt;
use std::fmt::Display;
use std::io;
use std::path::Path;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug)]
pub struct ParseError {
    pub path : Option<PathBuf>,
    pub line : String,
    pub col : usize,
}

impl ParseError {
    pub fn from_parts<P : Into<PathBuf>, S : Into<String>>(path: Option<P>, line: S, col: usize) -> Self {
        Self { path: path.map(|path| path.into()), line: line.into(), col }
    }
}

impl error::Error for ParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> { None }
}

impl Display for ParseError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref path) = self.path {
            write!(fmt, "{}: ", path.to_string_lossy())?;
        }
        write!(fmt, "'{}', error at column {}", self.line, self.col)
    }
}

#[derive(Debug)]
pub struct IoError {
    pub path : Option<PathBuf>,
    pub source : io::Error,
}

impl IoError {
    pub fn without_path(source: io::Error) -> Self {
        Self { path: None, source }
    }

    pub fn from_parts<P : Into<PathBuf>>(path: Option<P>, source: io::Error) -> Self {
        Self { path: path.map(|path| path.into()), source }
    }
}

impl error::Error for IoError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&self.source)
    }
}

impl Display for IoError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref path) = self.path {
            write!(fmt, "{}: ", path.to_string_lossy())?;
        }
        write!(fmt, "{}", self.source)
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    Parse(ParseError),
    Io(IoError),
    EnvVar(env::VarError),
}

impl From<ParseError> for Error {
    fn from(parse: ParseError) -> Self { Self::Parse(parse) }
}

impl From<IoError> for Error {
    fn from(io: IoError) -> Self { Self::Io(io) }
}

impl From<env::VarError> for Error {
    fn from(source: env::VarError) -> Self { Self::EnvVar(source) }
}

impl Error {
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Parse(source) => source.path.as_ref(),
            Self::Io(io) => io.path.as_ref(),
            Self::EnvVar(_) => None,
        }
        .map(|path| path.as_path())
    }

    pub fn not_found(&self) -> bool {
        if let Error::Io(ref io) = *self {
            return io.source.kind() == io::ErrorKind::NotFound;
        }
        false
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Io(io) => Some(&io.source),
            Error::EnvVar(ref source) => Some(source),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(io) => write!(fmt, "{io}"),
            Error::EnvVar(source) => write!(fmt, "{source}"),
            Error::Parse(source) => write!(fmt, "{source}"),
        }
    }
}

#[cfg(test)]
mod test {
    use std::env;
    use std::error::Error as StdError;
    use std::io;

    use super::*;

    const TEST_ENV_PATH : &'static str = "path/to/.env";

    #[test]
    fn test_io_error_source() {
        let path = PathBuf::from(TEST_ENV_PATH);
        let source = io::ErrorKind::PermissionDenied.into();
        let err : Error = IoError::from_parts(path.into(), source).into();
        let io_err = err.source().unwrap().downcast_ref::<io::Error>().unwrap();
        assert_eq!(io::ErrorKind::PermissionDenied, io_err.kind());
    }

    #[test]
    fn test_envvar_error_source() {
        let err : Error = env::VarError::NotPresent.into();
        let var_err = err
            .source()
            .unwrap()
            .downcast_ref::<env::VarError>()
            .unwrap();
        assert_eq!(&env::VarError::NotPresent, var_err);
    }

    #[test]
    fn test_lineparse_error_source() {
        let path = PathBuf::from(TEST_ENV_PATH);
        let line = "test line".to_string();
        let col = 2;
        let err : Error = ParseError::from_parts(path.into(), line, col).into();
        assert!(err.source().is_none());
    }

    #[test]
    fn test_error_not_found_true() {
        let path = PathBuf::from(TEST_ENV_PATH);
        let source = io::ErrorKind::NotFound.into();
        let err : Error = IoError::from_parts(path.into(), source).into();
        assert!(err.not_found());
    }

    #[test]
    fn test_error_not_found_false() {
        let path = PathBuf::from(TEST_ENV_PATH);
        let source = io::ErrorKind::PermissionDenied.into();
        let err : Error = IoError::from_parts(path.into(), source).into();
        assert!(!err.not_found());
    }

    #[test]
    fn test_io_error_display() {
        let path = PathBuf::from(TEST_ENV_PATH);
        let source = io::ErrorKind::PermissionDenied.into();
        let expected_desc = format!("{}: {source}", path.to_string_lossy());
        let err : Error = IoError::from_parts(path.clone().into(), source).into();
        assert_eq!(expected_desc, format!("{err}"));
    }

    #[test]
    fn test_envvar_error_display() {
        let var_err = env::VarError::NotPresent;
        let err : Error = var_err.clone().into();
        assert_eq!(format!("{err}"), format!("{var_err}"));
    }

    #[test]
    fn test_lineparse_error_display() {
        let path = PathBuf::from(TEST_ENV_PATH);
        let line = "test line".to_string();
        let col = 2;
        let err : Error = ParseError::from_parts(path.into(), line, col).into();
        let err_desc = format!("{}", err);
        assert_eq!(
            "path/to/.env: 'test line', error at column 2",
            err_desc
        );
    }
}
