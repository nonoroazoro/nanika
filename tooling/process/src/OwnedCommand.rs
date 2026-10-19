use std::io;
use std::process::{Child, Command};

use nanika_platform::{ProcessTree, configure_process_command};

/// The supervisor owns containment independently of the command leader's lifetime.
pub(crate) struct OwnedCommand {
    pub(crate) child: Child,
    pub(crate) tree: ProcessTree
}

impl OwnedCommand {
    pub(crate) fn spawn(mut command: Command) -> io::Result<Self> {
        configure_process_command(&mut command);
        let mut child = command.spawn()?;
        match ProcessTree::attach_workflow(&child) {
            Ok(tree) => Ok(Self { child, tree }),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                Err(error)
            }
        }
    }
}

impl Drop for OwnedCommand {
    fn drop(&mut self) {
        if let Err(error) = self.tree.terminate(self.child.id()) {
            eprintln!("Could not terminate owned process tree: {error}");
        }
        if let Err(error) = self.child.wait() {
            eprintln!("Could not reap owned command: {error}");
        }
    }
}
