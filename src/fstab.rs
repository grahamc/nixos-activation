use regex::Regex;

lazy_static! {
    static ref COMMENT_REMOVAL_REGEXP: Regex = Regex::new(
        r"^\s*#.*$"
    ).unwrap();
}


#[derive(Debug, PartialEq)]
pub struct FSTabFile<'a> {
    pub entries: Vec<FSTabEntry<'a>>,
}

/// A filesystem line
#[derive(Debug, PartialEq)]
pub struct FSTabEntry<'a> {
    /// This field describes the block special device or
    /// filesystem to be mounted.
    pub spec: &'a str,

    /// This field  describes the mount point (target) for the
    /// filesystem.
    pub file: &'a str,

    /// This field describes the type of the filesystem.
    pub fs_type: &'a str,

    /// This field describes the mount options associated with the
    /// filesystem, in a comma separated list.
    pub options: &'a str,

    /// This field is used by dump(8) to determine which filesystems
    /// need to be dumped. Defaults to 0.
    pub dump: i8,

    /// This field is used by fsck(8) to determine the order in which
    /// filesystem checks are done at boot time. Defaults to 0.
    pub fsck_pass: i8,
}

/// Parse a single line of an fstab
///
/// According to `man fstab` each line is a series of space-separated
/// fields. Leading spaces are ignored. Lines starting with a `#` are
/// skipped.
///
/// According to the source code (libmount/src/tab_parse.c) invalid
/// lines are simply skipped.
///
/// Note: According to the documentation, an fstab's `file` field can
/// contain spaces and tabs if they are represented by \040 and \011.
/// This function doesn't decode these octal characters.
pub fn parse_fstab_line<'a>(fstab: &'a str) -> Option<FSTabEntry<'a>> {
    if COMMENT_REMOVAL_REGEXP.is_match(fstab) {
        return None
    }

    let mut parts = fstab.split_whitespace();
    let result = Some(FSTabEntry {
        spec: parts.next()?,
        file: parts.next()?,
        fs_type: parts.next()?,

        // "options" is required by the manual, but it seems they can
        // be ommitted based on the util-linux source
        // see: libmount/src/tab_parse.c
        options: parts.next().unwrap_or(""),
        dump: parts.next().unwrap_or("0").parse::<i8>().unwrap_or(0),
        fsck_pass: parts.next().unwrap_or("0").parse::<i8>().unwrap_or(0),
    });
    if parts.next() == None {
        return result
    } else {
        return None
    }
}

/// Pass in an iterator of ftab lines, ie: "my\nfile".lines()
/// and get back a parsed representation of the file.
///
/// See parse_fstab_line for more information about edge cases and
/// specific behavior of this implementation.
pub fn parse_fstab<'a, T: Iterator<Item = &'a str>>(fstab_lines: T) -> FSTabFile<'a> {
    FSTabFile {
        entries: fstab_lines
            .map(|line| parse_fstab_line(line))
            .filter_map(|x|x).collect::<Vec<FSTabEntry>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fstab_line_comment() {
        assert_eq!(
            parse_fstab_line("# This is a generated file.  Do not edit!"),
            None
        );
        assert_eq!(
            parse_fstab_line("       # This is a generated file.  Do not edit!"),
            None
        );
    }

    #[test]
    fn parse_fstab_line_valid() {
        assert_eq!(
            parse_fstab_line("/dev/disk/by-uuid/3aa72460-7d05-4bd4-861f-6ef8b82082dc / ext4 defaults 0 1"),
            Some(FSTabEntry {
                spec: "/dev/disk/by-uuid/3aa72460-7d05-4bd4-861f-6ef8b82082dc",
                file: "/",
                fs_type: "ext4",
                options: "defaults",
                dump: 0,
                fsck_pass: 1
            })
        );
    }

    #[test]
    fn parse_fstab_line_valid_default_opts() {
        assert_eq!(
            parse_fstab_line("/dev/disk/by-uuid/3aa72460-7d05-4bd4-861f-6ef8b82082dc / ext4 defaults"),
            Some(FSTabEntry {
                spec: "/dev/disk/by-uuid/3aa72460-7d05-4bd4-861f-6ef8b82082dc",
                file: "/",
                fs_type: "ext4",
                options: "defaults",
                dump: 0,
                fsck_pass: 0,
            })
        );
    }

    #[test]
    fn parse_fstab_line_valid_swap() {
        assert_eq!(
            parse_fstab_line("/dev/disk/by-uuid/102799bd-d9d2-4ef6-936f-6ba9b59f168e none swap"),
            Some(FSTabEntry {
                spec: "/dev/disk/by-uuid/102799bd-d9d2-4ef6-936f-6ba9b59f168e",
                file: "none",
                fs_type: "swap",
                options: "",
                dump: 0,
                fsck_pass: 0,
            })
        );
    }

    #[test]
    fn parse_fstab_line_invalid_too_many_fields() {
        assert_eq!(
            parse_fstab_line("/dev/disk/by-uuid/3aa72460-7d05-4bd4-861f-6ef8b82082dc / ext4 defaults 0 1 # foo # bar"),
            None,
        );
    }

    #[test]
    fn parse_fstab_comments_and_blank_lines() {
        assert_eq!(
            parse_fstab("
                # This is a generated file.  Do not edit!
                #
                # To make changes, edit the fileSystems and swapDevices NixOS options
                # in your /etc/nixos/configuration.nix file.

                # Filesystems.

                # Swap devices.
            ".lines()),
            FSTabFile {
                entries: vec![],
            },
        );
    }

    #[test]
    fn parse_fstab_morbo() {
        assert_eq!(
            parse_fstab("
                # This is a generated file.  Do not edit!
                #
                # To make changes, edit the fileSystems and swapDevices NixOS options
                # in your /etc/nixos/configuration.nix file.

                # Filesystems.
                /dev/disk/by-uuid/3aa72460-7d05-4bd4-861f-6ef8b82082dc / ext4 defaults 0 1
                /dev/disk/by-uuid/2D03-B634 /boot vfat defaults 0 2


                # Swap devices.
                /dev/disk/by-uuid/102799bd-d9d2-4ef6-936f-6ba9b59f168e none swap
            ".lines()),
            FSTabFile {
                entries: vec![
                    FSTabEntry {
                        spec: "/dev/disk/by-uuid/3aa72460-7d05-4bd4-861f-6ef8b82082dc",
                        file: "/",
                        fs_type: "ext4",
                        options: "defaults",
                        dump: 0,
                        fsck_pass: 1
                    },
                    FSTabEntry {
                        spec: "/dev/disk/by-uuid/2D03-B634",
                        file: "/boot",
                        fs_type: "vfat",
                        options: "defaults",
                        dump: 0,
                        fsck_pass: 2
                    },
                    FSTabEntry {
                        spec: "/dev/disk/by-uuid/102799bd-d9d2-4ef6-936f-6ba9b59f168e",
                        file: "none",
                        fs_type: "swap",
                        options: "",
                        dump: 0,
                        fsck_pass: 0
                    },
                ],
            },
        );
    }

    #[test]
    fn parse_fstab_utillinux_fstab_comment() {
        // This fstab example is from util-linux 2.32.1's
        // tests/ts/libmount/files/fstab.comment
        assert_eq!(
            parse_fstab("
#
 # this is a leading comment
#

# this comments belongs to the first fs
UUID=d3a8f783-df75-4dc8-9163-975a891052c0 /     ext3    noatime,defaults 1 1
UUID=fef7ccb3-821c-4de8-88dc-71472be5946f /boot ext3    noatime,defaults 1 2

# 3rd fs comment + newline padding

UUID=1f2aa318-9c34-462e-8d29-260819ffd657 swap  swap    defaults        0 0
tmpfs                   /dev/shm                tmpfs   defaults        0 0
devpts                  /dev/pts                devpts  gid=5,mode=620  0 0
sysfs                   /sys                    sysfs   defaults        0 0
proc                    /proc                   proc    defaults        0 0
# this is comment
/dev/mapper/foo		/home/foo              ext4	noatime,defaults 0 0
foo.com:/mnt/share	/mnt/remote		nfs	noauto
//bar.com/gogogo        /mnt/gogogo             cifs    user=SRGROUP/baby,noauto
/dev/foo		/any/foo/		auto	defaults 0 0

#this is a trailing comment
            ".lines()),
            FSTabFile {
                entries: vec![
                    FSTabEntry {
                        spec: "UUID=d3a8f783-df75-4dc8-9163-975a891052c0",
                        file: "/",
                        fs_type: "ext3",
                        options: "noatime,defaults",
                        dump: 1,
                        fsck_pass: 1
                    },
                    FSTabEntry {
                        spec: "UUID=fef7ccb3-821c-4de8-88dc-71472be5946f",
                        file: "/boot",
                        fs_type: "ext3",
                        options: "noatime,defaults",
                        dump: 1,
                        fsck_pass: 2
                    },
                    FSTabEntry {
                        spec: "UUID=1f2aa318-9c34-462e-8d29-260819ffd657",
                        file: "swap",
                        fs_type: "swap",
                        options: "defaults",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "tmpfs",
                        file: "/dev/shm",
                        fs_type: "tmpfs",
                        options: "defaults",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "devpts",
                        file: "/dev/pts",
                        fs_type: "devpts",
                        options: "gid=5,mode=620",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "sysfs",
                        file: "/sys",
                        fs_type: "sysfs",
                        options: "defaults",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "proc",
                        file: "/proc",
                        fs_type: "proc",
                        options: "defaults",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "/dev/mapper/foo",
                        file: "/home/foo",
                        fs_type: "ext4",
                        options: "noatime,defaults",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "foo.com:/mnt/share",
                        file: "/mnt/remote",
                        fs_type: "nfs",
                        options: "noauto",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "//bar.com/gogogo",
                        file: "/mnt/gogogo",
                        fs_type: "cifs",
                        options: "user=SRGROUP/baby,noauto",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "/dev/foo",
                        file: "/any/foo/",
                        fs_type: "auto",
                        options: "defaults",
                        dump: 0,
                        fsck_pass: 0
                    },
                ],
            },
        );
    }

    #[test]
    fn parse_fstab_utillinux_fstab_broken() {
        // This fstab example is from util-linux 2.32.1's
        // tests/ts/libmount/files/fstab.broken
        assert_eq!(
            parse_fstab("
bug
UUID=d3a8f783-df75-4dc8-9163-975a891052c0 /     ext3    noatime,defaults 1 1
UUID=fef7ccb3-821c-4de8-88dc-71472be5946f /boot ext3    noatime,defaults 1 2
 UUID=1f2aa318-9c34-462e-8d29-260819ffd657 swap  swap    defaults        0 0
tmpfs                   /dev/shm                tmpfs   defaults        0 0
devpts                  /dev/pts                devpts  gid=5,mode=620
  sysfs                   /sys                    sysfs   defaults        0 0
this is broken line with unexpected number of fields
proc                    /proc                   proc    defaults        0 0
# this is comment
/dev/mapper/foo		/home/foo              ext4	noatime,defaults 1

foo.com:/mnt/share	/mnt/remote		nfs	noauto
//bar.com/gogogo        /mnt/gogogo             cifs    user=SRGROUP/baby,noauto
            ".lines()),
            FSTabFile {
                entries: vec![
                    FSTabEntry {
                        spec: "UUID=d3a8f783-df75-4dc8-9163-975a891052c0",
                        file: "/",
                        fs_type: "ext3",
                        options: "noatime,defaults",
                        dump: 1,
                        fsck_pass: 1
                    },
                    FSTabEntry {
                        spec: "UUID=fef7ccb3-821c-4de8-88dc-71472be5946f",
                        file: "/boot",
                        fs_type: "ext3",
                        options: "noatime,defaults",
                        dump: 1,
                        fsck_pass: 2
                    },
                    FSTabEntry {
                        spec: "UUID=1f2aa318-9c34-462e-8d29-260819ffd657",
                        file: "swap",
                        fs_type: "swap",
                        options: "defaults",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "tmpfs",
                        file: "/dev/shm",
                        fs_type: "tmpfs",
                        options: "defaults",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "devpts",
                        file: "/dev/pts",
                        fs_type: "devpts",
                        options: "gid=5,mode=620",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "sysfs",
                        file: "/sys",
                        fs_type: "sysfs",
                        options: "defaults",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "proc",
                        file: "/proc",
                        fs_type: "proc",
                        options: "defaults",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "/dev/mapper/foo",
                        file: "/home/foo",
                        fs_type: "ext4",
                        options: "noatime,defaults",
                        dump: 1,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "foo.com:/mnt/share",
                        file: "/mnt/remote",
                        fs_type: "nfs",
                        options: "noauto",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "//bar.com/gogogo",
                        file: "/mnt/gogogo",
                        fs_type: "cifs",
                        options: "user=SRGROUP/baby,noauto",
                        dump: 0,
                        fsck_pass: 0
                    },
                ],
            },
        );
    }


    #[test]
    fn parse_fstab_utillinux_fstab() {
        // This fstab example is from util-linux 2.32.1's
        // tests/ts/libmount/files/fstab
        assert_eq!(
            parse_fstab("
UUID=d3a8f783-df75-4dc8-9163-975a891052c0 /     ext3    noatime,defaults 1 1
UUID=fef7ccb3-821c-4de8-88dc-71472be5946f /boot ext3    noatime,defaults 1 2
UUID=1f2aa318-9c34-462e-8d29-260819ffd657 swap  swap    defaults        0 0
tmpfs                   /dev/shm                tmpfs   defaults        0 0
devpts                  /dev/pts                devpts  gid=5,mode=620  0 0
sysfs                   /sys                    sysfs   defaults        0 0
proc                    /proc                   proc    defaults        0 0
# this is comment
/dev/mapper/foo		/home/foo              ext4	noatime,defaults 0 0

foo.com:/mnt/share	/mnt/remote		nfs	noauto
//bar.com/gogogo        /mnt/gogogo             cifs    user=SRGROUP/baby,noauto

/dev/foo		/any/foo/		auto	defaults 0 0
            ".lines()),
            FSTabFile {
                entries: vec![
                    FSTabEntry {
                        spec: "UUID=d3a8f783-df75-4dc8-9163-975a891052c0",
                        file: "/",
                        fs_type: "ext3",
                        options: "noatime,defaults",
                        dump: 1,
                        fsck_pass: 1
                    },
                    FSTabEntry {
                        spec: "UUID=fef7ccb3-821c-4de8-88dc-71472be5946f",
                        file: "/boot",
                        fs_type: "ext3",
                        options: "noatime,defaults",
                        dump: 1,
                        fsck_pass: 2
                    },
                    FSTabEntry {
                        spec: "UUID=1f2aa318-9c34-462e-8d29-260819ffd657",
                        file: "swap",
                        fs_type: "swap",
                        options: "defaults",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "tmpfs",
                        file: "/dev/shm",
                        fs_type: "tmpfs",
                        options: "defaults",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "devpts",
                        file: "/dev/pts",
                        fs_type: "devpts",
                        options: "gid=5,mode=620",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "sysfs",
                        file: "/sys",
                        fs_type: "sysfs",
                        options: "defaults",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "proc",
                        file: "/proc",
                        fs_type: "proc",
                        options: "defaults",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "/dev/mapper/foo",
                        file: "/home/foo",
                        fs_type: "ext4",
                        options: "noatime,defaults",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "foo.com:/mnt/share",
                        file: "/mnt/remote",
                        fs_type: "nfs",
                        options: "noauto",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "//bar.com/gogogo",
                        file: "/mnt/gogogo",
                        fs_type: "cifs",
                        options: "user=SRGROUP/baby,noauto",
                        dump: 0,
                        fsck_pass: 0
                    },
                    FSTabEntry {
                        spec: "/dev/foo",
                        file: "/any/foo/",
                        fs_type: "auto",
                        options: "defaults",
                        dump: 0,
                        fsck_pass: 0
                    },
                ],
            },
        );
    }
}
