//! Http Status Codes
//! [RFC 9110 Section 15](https://www.rfc-editor.org/rfc/rfc9110#section-15)

use std::fmt::Display;

/// Http Status Codes
/// [RFC 9110 Section 15](https://www.rfc-editor.org/rfc/rfc9110#section-15)
#[repr(u16)]
#[derive(Debug, Clone, Copy)]
pub enum Status {
    /// 15.2.1
    Continue = 100,
    /// 15.2.2
    SwitchingProtocols = 101,
    /// 15.3.1
    r#Ok = 200,
    /// 15.3.2
    Created = 201,
    /// 15.3.3
    Accepted = 202,
    /// 15.3.4
    NonAuthoritativeInformation = 203,
    /// 15.3.5
    NoContent = 204,
    /// 15.3.6
    ResetContent = 205,
    /// 15.3.7
    PartialContent = 206,
    /// 15.4.1
    MultipleChoices = 300,
    /// 15.4.2
    MovedPermanently = 301,
    /// 15.4.3
    Found = 302,
    /// 15.4.4
    SeeOther = 303,
    /// 15.4.5
    NotModified = 304,
    /// 15.4.6
    UseProxy = 305,
    /// 15.4.8
    TemporaryRedirect = 307,
    /// 15.4.9
    PermanentRedirect = 308,
    /// 15.5.1
    BadRequest = 400,
    /// 15.5.2
    Unauthorized = 401,
    /// 15.5.3
    PaymentRequired = 402,
    /// 15.5.4
    Forbidden = 403,
    /// 15.5.5
    NotFound = 404,
    /// 15.5.6
    MethodNotAllowed = 405,
    /// 15.5.7
    NotAcceptable = 406,
    /// 15.5.8
    ProxyAuthenticationRequired = 407,
    /// 15.5.9
    RequestTimeout = 408,
    /// 15.5.10
    Conflict = 409,
    /// 15.5.11
    Gone = 410,
    /// 15.5.12
    LengthRequired = 411,
    /// 15.5.13
    PreconditionFailed = 412,
    /// 15.5.14
    ContentTooLarge = 413,
    /// 15.5.15
    UriTooLong = 414,
    /// 15.5.16
    UnsupportedMediaType = 415,
    /// 15.5.17
    RangeNotSatisfiable = 416,
    /// 15.5.18
    ExpectationFailed = 417,
    /// 15.5.20
    MisdirectedRequest = 421,
    /// 15.5.21
    UnprocessableContent = 422,
    /// 15.5.22
    UpgradeRequired = 426,
    /// 15.6.1
    InternalServerError = 500,
    /// 15.6.2
    NotImplemented = 501,
    /// 15.6.3
    BadGateway = 502,
    /// 15.6.4
    ServiceUnavailable = 503,
    /// 15.6.5
    GatewayTimeout = 504,
    /// 15.6.6
    HTTPVersionNotSupported = 505,
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", *self as u16))
    }
}
