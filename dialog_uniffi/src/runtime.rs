use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;
use dialog_lib::Dialog;

pub(crate) fn rt() -> &'static Runtime {
    static RT: OnceCell<Runtime> = OnceCell::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("dialog-uniffi")
            .build()
            .expect("Failed to create Tokio runtime")
    })
}

pub(crate) static DIALOG: OnceCell<Dialog> = OnceCell::new();

