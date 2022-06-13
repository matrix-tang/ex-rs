use tracing::{error, event, info, Level, span, instrument};

#[no_mangle]
#[inline(never)]
pub fn event() {
    tracing::info!("general informational messages relevant to users");
}

pub fn span() {
    let span = span!(Level::TRACE, "my_span");
    let _enter = span.enter();
    tracing::error!("SOMETHING IS SERIOUSLY WRONG!!!");
    tracing::warn!("important informational messages; might indicate an error");

    event!(Level::DEBUG, "something happened inside my_span");
    event!(Level::INFO, "something has happened 2!");
    // 设置了 target
    // 这里的对象位置分别是当前的 span 名和 target
    event!(target: "app_events",Level::INFO, "something has happened 3!");

    tracing::info!("general informational messages relevant to users");
}

#[instrument]
pub fn test_event() {
    info!("hello world event");
}

fn main() {
    tracing_subscriber::fmt()
        // all spans/events with a level higher than TRACE (e.g, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // sets this to be the default, global collector for this application.
        .init();
    event();

    error!("hello world");

    span();
    test_event();

    tracing::debug!("diagnostics used for internal debugging of a library or application");
    tracing::trace!("very verbose diagnostic events");

    event!(Level::INFO, answer = 42, question = "life, the universe, and everything");

    let user = "ferris";
    let s = span!(Level::TRACE, "login", user);
    let _enter = s.enter();

    info!(welcome="hello", user);

    // 字段名还可以使用字符串
    event!(Level::TRACE, "guid:x-request-id" = "abcdef", "type" = "request");

    #[derive(Debug)]
    struct MyStruct {
        field: &'static str,
    }

    let my_struct = MyStruct {
        field: "Hello world!",
    };

    // `my_struct` 将使用 Debug 的形式输出
    event!(Level::TRACE, greeting = ?my_struct);
    // 等价于:
    event!(Level::TRACE, greeting = tracing::field::debug(&my_struct));

    // `my_struct.field` 将使用 `fmt::Display` 的格式化形式输出
    event!(Level::TRACE, greeting = %my_struct.field);
    // 等价于:
    event!(Level::TRACE, greeting = tracing::field::display(&my_struct.field));

    // 作为对比，大家可以看下 Debug 和正常的字段输出长什么样
    event!(Level::TRACE, greeting = ?my_struct.field);
    event!(Level::TRACE, greeting = my_struct.field);
}