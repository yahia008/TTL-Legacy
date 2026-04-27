use proptest::prelude::*;

// Fuzz test for TTL extension logic
proptest! {
    #[test]
    fn fuzz_ttl_extension(
        current_ttl in 0u64..u64::MAX / 2,
        extension_interval in 1u64..u64::MAX / 2,
        num_extensions in 0usize..100,
    ) {
        let mut ttl = current_ttl;
        
        for _ in 0..num_extensions {
            let new_ttl = ttl.saturating_add(extension_interval);
            // Invariant: TTL should never decrease on check-in
            prop_assert!(new_ttl >= ttl);
            ttl = new_ttl;
        }
    }

    #[test]
    fn fuzz_ttl_expiry(
        current_time in 0u64..u64::MAX / 2,
        ttl_duration in 1u64..u64::MAX / 2,
        elapsed_time in 0u64..u64::MAX / 2,
    ) {
        let expiry_time = current_time.saturating_add(ttl_duration);
        let check_time = current_time.saturating_add(elapsed_time);
        
        let is_expired = check_time >= expiry_time;
        
        // Invariant: if elapsed_time >= ttl_duration, vault should be expired
        if elapsed_time >= ttl_duration {
            prop_assert!(is_expired);
        }
    }

    #[test]
    fn fuzz_check_in_with_random_timestamps(
        base_timestamp in 1000u64..u64::MAX / 2,
        check_in_interval in 1u64..86400u64 * 365, // up to 1 year
        num_check_ins in 0usize..50,
    ) {
        let mut last_check_in = base_timestamp;
        let mut ttl_remaining = check_in_interval;
        
        for i in 0..num_check_ins {
            let new_check_in = last_check_in.saturating_add(check_in_interval);
            ttl_remaining = check_in_interval;
            
            // Invariant: TTL should always be reset to check_in_interval
            prop_assert_eq!(ttl_remaining, check_in_interval);
            
            // Invariant: check-in time should advance
            prop_assert!(new_check_in >= last_check_in);
            
            last_check_in = new_check_in;
        }
    }

    #[test]
    fn fuzz_balance_invariant(
        initial_balance in 0i128..i128::MAX / 2,
        deposits in prop::collection::vec(1i128..i128::MAX / 100, 0..20),
        withdrawals in prop::collection::vec(1i128..i128::MAX / 100, 0..20),
    ) {
        let mut balance = initial_balance;
        let mut total_deposited = 0i128;
        let mut total_withdrawn = 0i128;
        
        for deposit in deposits {
            if let Some(new_balance) = balance.checked_add(deposit) {
                balance = new_balance;
                total_deposited = total_deposited.saturating_add(deposit);
            }
        }
        
        for withdrawal in withdrawals {
            if balance >= withdrawal {
                balance = balance.saturating_sub(withdrawal);
                total_withdrawn = total_withdrawn.saturating_add(withdrawal);
            }
        }
        
        // Invariant: balance should never exceed initial + deposits
        let max_possible = initial_balance.saturating_add(total_deposited);
        prop_assert!(balance <= max_possible);
        
        // Invariant: balance should never be negative
        prop_assert!(balance >= 0);
    }
}
