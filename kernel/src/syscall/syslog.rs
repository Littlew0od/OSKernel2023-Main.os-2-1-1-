/* Close the log.  Currently a NOP. */
pub const SYSLOG_ACTION_CLOSE: usize = 0;
/* Open the log. Currently a NOP. */
pub const SYSLOG_ACTION_OPEN: usize = 1;
/* Read from the log. */
pub const SYSLOG_ACTION_READ: usize = 2;
/* Read all messages remaining in the ring buffer. */
pub const SYSLOG_ACTION_READ_ALL: usize = 3;
/* Read and clear all messages remaining in the ring buffer */
pub const SYSLOG_ACTION_READ_CLEAR: usize = 4;
/* Clear ring buffer. */
pub const SYSLOG_ACTION_CLEAR: usize = 5;
/* Disable printk's to console */
pub const SYSLOG_ACTION_CONSOLE_OFF: usize = 6;
/* Enable printk's to console */
pub const SYSLOG_ACTION_CONSOLE_ON: usize = 7;
/* Set level of messages printed to console */
pub const SYSLOG_ACTION_CONSOLE_LEVEL: usize = 8;
/* Return number of unread characters in the log buffer */
pub const SYSLOG_ACTION_SIZE_UNREAD: usize = 9;
/* Return size of the log buffer */
pub const SYSLOG_ACTION_SIZE_BUFFER: usize = 10;

