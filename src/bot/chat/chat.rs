// The main reply cycle + lookup helpers. `reply` is a single method (~260 lines) that
// stays a bit over the 200-line guideline on purpose — splitting it mid-body would chop
// the function into fragments and hurt readability, defeating the point of that guideline.
include!("chat_reply.rs");
include!("chat_lookup.rs");
