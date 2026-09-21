fn main() {
    rhusky::Rhusky::new()
        .hooks_dir(".githooks")
        .skip_in_env("GITHUB_ACTIONS")
        .install_from_build_script()
        .expect("failed to install repository Git hooks");
}
