//! https://gitlab.com/apparmor/apparmor/-/blob/eb8f9302aa664e8ac84a03eaf11b1cb1372b1e44/profiles/apparmor/profiles/extras/usr.sbin.sshd

use anyhow::Result;
use rustable::medusa::{
    Config, ConfigBuilder, ConfigError, Connection, Context, HandlerArgs, HandlerFlags,
    MedusaAnswer, SpaceBuilder,
};
use rustable_codegen::handler;
use std::fs::OpenOptions;

const MEDUSA_FILE_NAME: &str = "/dev/medusa";

#[handler(subject_vs = "*", event = "getprocess", object_vs = "*")]
async fn getprocess_handler(ctx: &Context, args: HandlerArgs<'_>) -> Result<MedusaAnswer> {
    let evtype = args.evtype;
    let mut subject = args.subject;
    let cmdline = subject.get_attribute::<String>("cmdline")?;

    println!("cmdline = {cmdline}");

    if cmdline.contains("/usr/sbin/sshd") {
        subject
            .enter_tree(ctx, &evtype, "domains", "/usr/sbin/sshd")
            .await;
    } else if cmdline.contains("/usr/bin/passwd") {
        subject
            .enter_tree(ctx, &evtype, "domains", "/usr/bin/passwd")
            .await;
    } else {
        subject
            .enter_tree(ctx, &evtype, "domains", "/")
            .await;
    }

    subject.update(ctx).await;

    Ok(MedusaAnswer::Allow)
}

#[rustfmt::skip]
fn include_passwd(config: ConfigBuilder) -> ConfigBuilder {
    let mut reads = Vec::new();

    reads.push(SpaceBuilder::new()
        .with_name("usr_bin_passwd")
        .with_path(r"fs/usr/bin/passwd"));

    reads.push(SpaceBuilder::new()
        .with_name("pts-passwd")
        .with_path(r"fs/dev/pts/[0-9]*"));

    reads.push(SpaceBuilder::new()
        .with_name("run_utmp")
        .with_path(r"fs/run/utmp"));

    reads.push(SpaceBuilder::new()
        .with_name("var_run_utmp")
        .with_path(r"fs/var/run/utmp"));

    let read_names = reads.iter().map(|x| x.name());

    let passwd = SpaceBuilder::new()
        .with_name("passwd")
        .with_path("domains/usr/bin/passwd")
        .reads(read_names.clone())
        .writes(["pts", "run_utmp", "var_run_tmp"])
        .sees(read_names);

    config
        .add_space(passwd)
        .add_spaces(reads)
}

#[rustfmt::skip]
fn create_config() -> Result<Config, ConfigError> {
    let mut config = Config::builder();
    let mut reads = Vec::new();

    reads.push(SpaceBuilder::new()
        .with_name("ptmx")
        .with_path(r"fs/dev/ptmx"));

    reads.push(SpaceBuilder::new()
        .with_name("pts")
        .with_path(r"fs/dev/pts/[0-9]*"));

    reads.push(SpaceBuilder::new()
        .with_name("urandom")
        .with_path(r"fs/dev/urandom"));

    reads.push(SpaceBuilder::new()
        .with_name("locale")
        .with_path(r"fs/etc/default/locale"));

    reads.push(SpaceBuilder::new()
        .with_name("environment")
        .with_path(r"fs/etc/environment"));

    reads.push(SpaceBuilder::new()
        .with_name("modules")
        .with_path(r"fs/etc/modules.conf"));

    reads.push(SpaceBuilder::new()
        .with_name("security")
        .with_path_recursive(r"fs/etc/security"));

    reads.push(SpaceBuilder::new()
        .with_name("ssh")
        .with_path_recursive(r"fs/etc/ssh"));

    reads.push(SpaceBuilder::new()
        .with_name("openssl")
        .with_path(r"fs/etc/ssl/openssl.cnf"));

    reads.push(SpaceBuilder::new()
        .with_name("sbin_sshd")
        .with_path(r"fs/usr/sbin/sshd"));

    reads.push(SpaceBuilder::new()
        .with_name("btmp")
        .with_path(r"fs/var/log/btmp"));

    reads.push(SpaceBuilder::new()
        .with_name("authorized-keys")
        .with_path(r"fs/home/.*/\.ssh/authorized_keys(2?)"));

    reads.push(SpaceBuilder::new()
        .with_name("run_sshd")
        .with_path(r"fs/run/(sshd|sshd\.pid|sshd\.init\.pid)"));

    reads.push(SpaceBuilder::new()
        .with_name("cgroup")
        .with_path(r"fs/sys/fs/cgroup/.*/user/.*/[0-9]*"));

    reads.push(SpaceBuilder::new()
        .with_name("cgroup-systemd")
        .with_path(r"fs/sys/fs/cgroup/systemd/user\.slice/user-[0-9]*\.slice/session-c[0-9]*\.scope"));

    reads.push(SpaceBuilder::new()
        .with_name("bin_login-shell")
        .with_path(r"fs/bin/(ash|bash|bash2|bsh|csh|dash|ksh|sh|tcsh|zsh|zsh4|zsh5|false)"));

    reads.push(SpaceBuilder::new()
        .with_name("usr_bin_login-shell")
        .with_path(r"fs/usr/bin/(ash|bash|bash2|bsh|csh|dash|ksh|sh|tcsh|zsh|zsh4|zsh5|false)"));

    config = include_passwd(config);

    reads.push(SpaceBuilder::new()
        .with_name("sbin_nologin")
        .with_path(r"fs/sbin/nologin"));

    reads.push(SpaceBuilder::new()
        .with_name("usr_sbin_nologin")
        .with_path(r"fs/usr/sbin/nologin"));

    reads.push(SpaceBuilder::new()
        .with_name("legal")
        .with_path(r"fs/etc\.legal"));

    reads.push(SpaceBuilder::new()
        .with_name("motd")
        .with_path(r"fs/etc/motd"));

    reads.push(SpaceBuilder::new()
        .with_name("run_motd")
        .with_path(r"fs/run/motd(\.dynamic|\.new)?"));

    reads.push(SpaceBuilder::new()
        .with_name("var_run_motd")
        .with_path(r"fs/var/run/motd(\.dynamic|\.new)?"));

    let krb5cc = SpaceBuilder::new()
        .with_name("krb5cc")
        .with_path(r"fs/tmp/krb5cc.*");

    reads.push(SpaceBuilder::new()
        .with_name("tmp_ssh")
        .with_path_recursive(r"fs/tmp/ssh-[a-zA-Z0-9]*"));

    let read_names = reads.iter().map(|x| x.name());

    let sshd = SpaceBuilder::new()
        .with_name("sshd")
        .with_path("domains/usr/sbin/sshd")
        .reads(read_names.clone())
        .writes(["ptmx", "pts", "btmp", "cgroup", "cgroup-systemd", "run_motd",
                 "var_run_motd", "run_sshd", "krb5cc", "tmp_ssh"])
        .sees(read_names)
        .sees([krb5cc.name()]);

    let all_files = SpaceBuilder::new()
        .with_name("all_files")
        .with_path_recursive("fs/");

    let all_domains = SpaceBuilder::new()
        .with_name("all_domains")
        .with_path_recursive("domains/")
        .reads(["all_files", "all_domains"])
        .writes(["all_files", "all_domains"])
        .sees(["all_files", "all_domains"]);

    config
        .add_space(all_files)
        .add_space(all_domains)
        .add_space(krb5cc)
        .add_space(sshd)
        .add_spaces(reads)
        .add_hierarchy_event_handler("getfile", "fs", Some("filename"), HandlerFlags::FROM_OBJECT)
        .add_custom_event_handler(getprocess_handler)
        .build()
}

#[tokio::main]
async fn main() -> Result<()> {
    use anyhow::Context;
    let config = create_config().context("Failed to create config")?;

    let write_handle = OpenOptions::new()
        .read(true)
        .write(true)
        .open(MEDUSA_FILE_NAME)?;
    let read_handle = write_handle.try_clone()?;

    let mut connection = Connection::new(write_handle, read_handle, config)
        .await
        .context("Connection failed")?;
    connection.run().await.context("Communication failed")?;

    Ok(())
}
