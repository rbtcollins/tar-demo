# Demo for dealing with (most) IO latency in tar on Windows

This isn't as perfect as it could hypothetically be: it could for instance
mitigate create and write latency through fully event driven code, but this is
*substantially* harder to do : simple linear extraction of each file in the tar with async code does
not achieve it - that is just serialised in a different way.

What is needed is to queue every IO up in memory as they come out of the tar,
but only dispatch IO to the OS when the IO is able to be executed (effectively
forming userspace barriers around things like mkdir completion). Rustup has a
complete implementation of this that I wrote. It is however overkill: by the
time that you're not blocking on bulk writes (as it sits in the disk / OS
buffer) or closehandle latency (whether due to disk cache writes happening or
Defender scanning), it is usually possible to dispatch enough IO that that the
overall operation becomes bottlenecked on throughput rather than response time.

If there is enough interest, I'll happily do a clean standalone full-thing, but
for quick-and-understandable reading, this code base is more than sufficient.

Caveats:

- vulnerable to directory escaping bugs
- doesn't handle symlinks, device nodes or more exotic kinds
- not using extended-length paths: very deep (255 char) tar path + output root
  won't extract - easy to fix (\\?\ + the abspath with all separators in \
  form), but again, this is demo code, and clarity is more important than
  covering every edge case.
