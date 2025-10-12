[] Change every response filed from message to status_message

- Optymalizacje
    - Dodać indeksowanie user.id na direct chat user_a_id i user_b_id
       CREATE INDEX idx_direct_chats_user_a ON direct_chats(user_a_id)
        CREATE INDEX idx_direct_chats_user_b ON direct_chats(user_b_id)
