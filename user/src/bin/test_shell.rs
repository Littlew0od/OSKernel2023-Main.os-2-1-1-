#![no_std]
#![no_main]
#![allow(clippy::println_empty_string)]

extern crate alloc;

extern crate user_lib;

use alloc::string::String;
use alloc::vec::Vec;
use user_lib::*;

#[no_mangle]
pub fn main() -> i32 {
    println!("[user_shell] start test!");
    final2_test();
    println!("[user_shell] finish test!");
    shutdown(false)
}

pub fn load_final2_test_cmds() -> Vec<String> {
    let mut cmds = Vec::new();
    // cmds.push(String::from("./libc-bench"));
    cmds.push(String::from("./busybox sh ./busybox_testcode.sh"));
    // cmds.push(String::from("./busybox sh ./lua_testcode.sh"));
    // cmds.push(String::from("./time-test"));
    // cmds.push(String::from("./busybox sh ./run-static.sh"));
    // cmds.push(String::from("./busybox sh ./run-dynamic.sh"));
    // cmds.push(String::from("./busybox sh ./iozone_testcode.sh"));
    // cmds.push(String::from("./busybox sh ./lmbench_testcode.sh"));
    // cmds.push(String::from("./busybox sh ./unixbench_testcode.sh"));
    // cmds.push(String::from("./busybox sh ./iperf_testcode.sh"));
    // cmds.push(String::from("./busybox sh ./cyclictest_testcode.sh"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe pthread_cancel_points"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe pthread_cancel"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe pthread_cond"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe pthread_tsd"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe pthread_robust_detach"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe pthread_cancel_sem_wait"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe pthread_cond_smasher"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe pthread_condattr_setclock"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe pthread_exit_cancel"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe pthread_once_deadlock"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe pthread_rwlock_ebusy"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe argv"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe basename"));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe clocale_mbfuncs",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe clock_gettime",
    // ));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe crypt"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe dirname"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe env"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe fdopen"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe fnmatch"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe fscanf"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe fwscanf"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe iconv_open"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe inet_pton"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe mbc"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe memstream"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe qsort"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe random"));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe search_hsearch",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe search_insque",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe search_lsearch",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe search_tsearch",
    // ));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe setjmp"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe snprintf"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe sscanf"));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe sscanf_long",
    // ));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe stat"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe strftime"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe string"));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe string_memcpy",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe string_memmem",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe string_memset",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe string_strchr",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe string_strcspn",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe string_strstr",
    // ));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe strptime"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe strtod"));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe strtod_simple",
    // ));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe strtof"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe strtol"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe strtold"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe swprintf"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe tgmath"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe time"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe tls_align"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe udiv"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe ungetc"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe utime"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe wcsstr"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe wcstol"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe pleval"));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe daemon_failure",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe dn_expand_empty",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe dn_expand_ptr_0",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe fflush_exit",
    // ));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe fgets_eof"));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe fgetwc_buffering",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe fpclassify_invalid_ld80",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe ftello_unflushed_append",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe getpwnam_r_crash",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe getpwnam_r_errno",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe iconv_roundtrips",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe inet_ntop_v4mapped",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe inet_pton_empty_last_field",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe iswspace_null",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe lrand48_signextend",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe lseek_large",
    // ));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe malloc_0"));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe mbsrtowcs_overflow",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe memmem_oob_read",
    // ));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe memmem_oob"));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe mkdtemp_failure",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe mkstemp_failure",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe printf_1e9_oob",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe printf_fmt_g_round",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe printf_fmt_g_zeros",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe printf_fmt_n",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe putenv_doublefree",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe regex_backref_0",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe regex_bracket_icase",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe regex_ere_backref",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe regex_escaped_high_byte",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe regex_negated_range",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe regexec_nosub",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe rewind_clear_error",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe rlimit_open_files",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe scanf_bytes_consumed",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe scanf_match_literal_eof",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe scanf_nullbyte_char",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe setvbuf_unget",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe sigprocmask_internal",
    // ));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe sscanf_eof"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe statvfs"));
    // cmds.push(String::from("./runtest.exe -w entry-static.exe strverscmp"));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe syscall_sign_extend",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe uselocale_0",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe wcsncpy_read_overflow",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-static.exe wcsstr_false_negative",
    // ));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe argv"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe basename"));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe clocale_mbfuncs",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe clock_gettime",
    // ));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe crypt"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe dirname"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe dlopen"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe env"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe fdopen"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe fnmatch"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe fscanf"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe fwscanf"));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe iconv_open",
    // ));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe inet_pton"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe mbc"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe memstream"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe qsort"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe random"));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe search_hsearch",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe search_insque",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe search_lsearch",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe search_tsearch",
    // ));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe setjmp"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe snprintf"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe sscanf"));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe sscanf_long",
    // ));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe stat"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe strftime"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe string"));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe string_memcpy",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe string_memmem",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe string_memset",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe string_strchr",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe string_strcspn",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe string_strstr",
    // ));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe strptime"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe strtod"));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe strtod_simple",
    // ));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe strtof"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe strtol"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe strtold"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe swprintf"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe tgmath"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe time"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe udiv"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe ungetc"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe utime"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe wcsstr"));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe wcstol"));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe daemon_failure",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe dn_expand_empty",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe dn_expand_ptr_0",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe fflush_exit",
    // ));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe fgets_eof"));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe fgetwc_buffering",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe fpclassify_invalid_ld80",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe ftello_unflushed_append",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe getpwnam_r_crash",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe getpwnam_r_errno",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe iconv_roundtrips",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe inet_ntop_v4mapped",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe inet_pton_empty_last_field",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe iswspace_null",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe lrand48_signextend",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe lseek_large",
    // ));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe malloc_0"));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe mbsrtowcs_overflow",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe memmem_oob_read",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe memmem_oob",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe mkdtemp_failure",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe mkstemp_failure",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe printf_1e9_oob",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe printf_fmt_g_round",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe printf_fmt_g_zeros",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe printf_fmt_n",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe putenv_doublefree",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe regex_backref_0",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe regex_bracket_icase",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe regex_ere_backref",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe regex_escaped_high_byte",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe regex_negated_range",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe regexec_nosub",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe rewind_clear_error",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe rlimit_open_files",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe scanf_bytes_consumed",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe scanf_match_literal_eof",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe scanf_nullbyte_char",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe setvbuf_unget",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe sigprocmask_internal",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe sscanf_eof",
    // ));
    // cmds.push(String::from("./runtest.exe -w entry-dynamic.exe statvfs"));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe strverscmp",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe syscall_sign_extend",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe uselocale_0",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe wcsncpy_read_overflow",
    // ));
    // cmds.push(String::from(
    //     "./runtest.exe -w entry-dynamic.exe wcsstr_false_negative",
    // ));

    cmds
}

pub fn final2_test() {
    let cmds = load_final2_test_cmds();

    for cmd in cmds {
        let (args_copy, args_addr) = str2args(&cmd);
        let pid = fork();
        if pid == 0 {
            exec(args_copy[0].as_str(), args_addr.as_slice());
        } else {
            let mut exit_code = 0;
            waitpid(pid as usize, &mut exit_code);
        }
    }
}

pub fn str2args(s: &str) -> (Vec<String>, Vec<*const u8>) {
    let args_copy: Vec<String> = s
        .split(' ')
        .map(|s1| {
            let mut string = String::new();
            string.push_str(&s1);
            string.push('\0');
            string
        })
        .collect();

    let mut args_addr: Vec<*const u8> = args_copy.iter().map(|arg| arg.as_ptr()).collect();
    args_addr.push(core::ptr::null::<u8>());

    (args_copy, args_addr)
}
