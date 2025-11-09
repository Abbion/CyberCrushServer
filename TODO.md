[] Change every entry in documentation that responds to response status
[] Check for error format [ Error: Example text ]

Version 2
[] Change response_status arguments from String to str, to avoid using .into()
[] Change random generated token to JWT
[] In reltime chat server change the way we update the last message. Use a database event to automaticaly update the last metadaa for a chat when a new message is added
[] In realtime chat use only one way to send error messages
[] In project with a lot of request and response structs create a seperate file that stores only those structures
[] Change all errors to print user_id and not user token. Connected to JWT
[] In common for chat server create a function called check_membership to use across chat server
[] Check for max members in group chat on server side too
[] Add possibility to change accounts fast
[] 

Version ?
[] Image transfer implementation

