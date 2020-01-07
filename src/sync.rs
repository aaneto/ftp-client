//! The blocking implementation of the client.
use crate::client::Client as AsyncClient;
use crate::client::ClientMode;
use tokio::runtime::Runtime;

/// A wrapper over the async client.
pub struct Client {
    inner_client: AsyncClient,
    runtime: Runtime,
}

impl Client {
    /// Set the mode for the client.
    pub fn set_mode(&mut self, mode: ClientMode) {
        self.inner_client.set_mode(mode)
    }

    /// Connect to a new FTP server using plain text (no TLS).
    pub fn connect(
        hostname: &str,
        user: &str,
        password: &str,
    ) -> Result<Self, crate::error::Error> {
        let mut runtime = Runtime::new().unwrap();
        let inner_client =
            runtime.block_on(AsyncClient::connect_with_port(hostname, 21, user, password))?;

        Ok(Client {
            inner_client,
            runtime,
        })
    }

    /// Connect to a new FTP server using a secure connection (TLS).
    pub fn connect_tls(
        hostname: &str,
        user: &str,
        password: &str,
    ) -> Result<Self, crate::error::Error> {
        let mut runtime = Runtime::new().unwrap();
        let inner_client = runtime.block_on(AsyncClient::connect_tls_with_port(
            hostname, 21, user, password,
        ))?;

        Ok(Client {
            inner_client,
            runtime,
        })
    }

    /// Get the welcome message sent by the server at the connection establishment.
    pub fn get_welcome(&self) -> Option<&String> {
        self.inner_client.get_welcome()
    }

    /// Login using the given user and password.
    /// Note that many servers require a login with an anonymous user,
    /// such as client.login("anonymous", "anonymous@mail.com").
    pub fn login(&mut self, user: &str, password: &str) -> Result<(), crate::error::Error> {
        self.runtime
            .block_on(self.inner_client.login(user, password))
    }

    /// Logout from the current user/password pair.
    pub fn logout(&mut self) -> Result<(), crate::error::Error> {
        self.runtime.block_on(self.inner_client.logout())
    }

    /// Change the working directory on the current session.
    pub fn cwd(&mut self, dir: &str) -> Result<(), crate::error::Error> {
        self.runtime.block_on(self.inner_client.cwd(dir))
    }

    /// Go up to the parent directory on the current session.
    pub fn cdup(&mut self) -> Result<(), crate::error::Error> {
        self.runtime.block_on(self.inner_client.cdup())
    }

    /// Show server information regarding its implementation status
    /// to the user.
    ///
    /// The help command can also be used with an argument to see detailed
    /// information about a single command, this behaviour is not implemented.
    pub fn help(&mut self) -> Result<(), crate::error::Error> {
        self.runtime.block_on(self.inner_client.help())
    }

    /// This command should not do anything other than receiving
    /// an OK response from the server.
    pub fn noop(&mut self) -> Result<(), crate::error::Error> {
        self.runtime.block_on(self.inner_client.noop())
    }

    /// Set the transfer type to ascii
    pub fn ascii(&mut self) -> Result<(), crate::error::Error> {
        self.runtime.block_on(self.inner_client.ascii())
    }

    /// Set the transfer type to binary
    pub fn binary(&mut self) -> Result<(), crate::error::Error> {
        self.runtime.block_on(self.inner_client.binary())
    }

    /// Get the current reported status from the server. This can be used
    /// during transfer and between them. This command can be used with
    /// and argument to get behaviour similar to LIST, this particular
    /// behaviour is not implemented.
    pub fn status(&mut self) -> Result<String, crate::error::Error> {
        self.runtime.block_on(self.inner_client.status())
    }

    /// List the provided path in any way the server desires.
    pub fn list(&mut self, path: &str) -> Result<String, crate::error::Error> {
        self.runtime.block_on(self.inner_client.list(path))
    }

    /// List the provided path, providing only name information about files and directories.
    pub fn list_names(&mut self, path: &str) -> Result<Vec<String>, crate::error::Error> {
        self.runtime.block_on(self.inner_client.list_names(path))
    }

    /// Store a new file on a provided path and name.
    pub fn store<B: AsRef<[u8]>>(
        &mut self,
        path: &str,
        data: B,
    ) -> Result<(), crate::error::Error> {
        self.runtime.block_on(self.inner_client.store(path, data))
    }

    /// Store a new file on a provided path using a random unique name.
    pub fn store_unique<B: AsRef<[u8]>>(&mut self, data: B) -> Result<String, crate::error::Error> {
        self.runtime.block_on(self.inner_client.store_unique(data))
    }

    /// Append to a existing file or a create a new one.
    pub fn append<B: AsRef<[u8]>>(
        &mut self,
        path: &str,
        data: B,
    ) -> Result<(), crate::error::Error> {
        self.runtime.block_on(self.inner_client.append(path, data))
    }

    /// Restart a file transfer. Unimplemented.
    pub fn restart(&mut self) -> Result<(), crate::error::Error> {
        unimplemented!();
    }

    /// Abort a file transfer. Unimplemented.
    pub fn abort(&mut self) -> Result<(), crate::error::Error> {
        unimplemented!();
    }

    /// Preallocate space on the server. Unimplemented.
    pub fn allocate(
        &mut self,
        _logical_size: usize,
        _logical_page_size: Option<usize>,
    ) -> Result<(), crate::error::Error> {
        unimplemented!();
    }

    /// Move a file from a path to another, essentially renaming it.
    pub fn rename_file(
        &mut self,
        path_from: &str,
        path_to: &str,
    ) -> Result<(), crate::error::Error> {
        self.runtime
            .block_on(self.inner_client.rename_file(path_from, path_to))
    }

    /// Remove an existing directory.
    pub fn remove_directory(&mut self, dir_path: &str) -> Result<(), crate::error::Error> {
        self.runtime
            .block_on(self.inner_client.remove_directory(dir_path))
    }

    /// Make a new directory.
    pub fn make_directory(&mut self, dir_path: &str) -> Result<(), crate::error::Error> {
        self.runtime
            .block_on(self.inner_client.make_directory(dir_path))
    }

    /// Get the current working directory.
    pub fn pwd(&mut self) -> Result<String, crate::error::Error> {
        self.runtime.block_on(self.inner_client.pwd())
    }

    /// This command is used by the server to provide services
    /// specific to his system that are essential to file transfer
    /// but not sufficiently universal to be included as commands in
    /// the protocol.
    ///
    /// The nature of these services and the
    /// specification of their syntax can be stated in a reply to
    /// the HELP SITE command.
    ///
    /// Extracted from RFC959.
    pub fn site_parameters(&mut self) -> Result<String, crate::error::Error> {
        self.runtime.block_on(self.inner_client.site_parameters())
    }

    /// Get the type of operating system on the server.
    pub fn system(&mut self) -> Result<String, crate::error::Error> {
        self.runtime.block_on(self.inner_client.system())
    }

    /// Delete a file at a path.
    pub fn delete_file(&mut self, dir_path: &str) -> Result<(), crate::error::Error> {
        self.runtime
            .block_on(self.inner_client.delete_file(dir_path))
    }

    /// Download a file at a path into a byte buffer.
    pub fn retrieve_file(&mut self, path: &str) -> Result<Vec<u8>, crate::error::Error> {
        self.runtime.block_on(self.inner_client.retrieve_file(path))
    }
}
