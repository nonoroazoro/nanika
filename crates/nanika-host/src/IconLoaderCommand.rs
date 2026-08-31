use crate::IconIdentity;

pub(crate) enum IconLoaderCommand {
    Load(IconIdentity),
    Shutdown,
}
