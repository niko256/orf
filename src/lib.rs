pub mod commands {
    pub mod core {
        pub mod add;
        pub mod catfile;
        pub mod commit;
        pub mod hash_object;
        pub mod init;
        pub mod status;
        pub mod write_tree;

        pub mod index {
            pub mod idx_ls;
            pub mod idx_main;
            pub mod idx_rm;
        }
    }

    pub mod history {
        pub mod diff;
        pub mod log;
        pub mod show;
    }

    pub mod remote {
        pub mod remote;

        pub mod clone {
            pub mod checkout_phase;
            pub mod fetch_phase;
        }
    }

    pub mod config {
        pub mod conf_utils;
        pub mod config;
    }

    pub mod branching {
        pub mod branch;
        pub mod checkout;
    }
}

/// --------------------------------------------------------------

pub mod storage {
    pub mod refs;
    pub mod repo;
    pub mod utils;

    pub mod objects {
        pub mod blob;
        pub mod branch;
        pub mod change;
        pub mod commit;
        pub mod conflict;
        pub mod delta;
        pub mod pack;
        pub mod tag;
        pub mod tree;
    }
}
