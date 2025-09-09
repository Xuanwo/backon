Implement a budget-based retry strategy that prevents retry storms.

This example demonstrates how to implement a budget-based backoff strategy using backon's existing API.
The budget system works by maintaining a pool of retry "tokens" that are:
- Withdrawn when a retry is attempted
- Deposited back when an operation succeeds
- Prevents further retries when exhausted

```rust
use backon::ExponentialBuilder;
use backon::Retryable;
use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

/// A simple retry budget that tracks available retry attempts.
/// 
/// This implementation is inspired by Tower's retry budget concept,
/// which helps prevent retry storms during system failures.
#[derive(Clone)]
struct RetryBudget {
    /// Available budget for retries
    balance: Arc<AtomicI32>,
    /// Amount to withdraw for each retry attempt
    withdraw_amount: i32,
    /// Amount to deposit on successful completion
    deposit_amount: i32,
}

impl RetryBudget {
    fn new(initial_balance: i32) -> Self {
        Self {
            balance: Arc::new(AtomicI32::new(initial_balance)),
            withdraw_amount: 1,
            deposit_amount: 1,
        }
    }

    /// Attempt to withdraw from the budget for a retry.
    /// Returns true if withdrawal succeeded, false if insufficient budget.
    fn withdraw(&self) -> bool {
        let mut current = self.balance.load(Ordering::Relaxed);
        loop {
            if current < self.withdraw_amount {
                return false;
            }
            match self.balance.compare_exchange_weak(
                current,
                current - self.withdraw_amount,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(x) => current = x,
            }
        }
    }

    /// Deposit back to the budget after a successful operation.
    fn deposit(&self) {
        self.balance.fetch_add(self.deposit_amount, Ordering::SeqCst);
    }

    /// Get the current budget balance.
    fn balance(&self) -> i32 {
        self.balance.load(Ordering::Relaxed)
    }
}

/// A potentially unreliable service that we want to retry with budget control.
struct UnreliableService {
    attempt: Arc<AtomicI32>,
}

impl UnreliableService {
    fn new() -> Self {
        Self {
            attempt: Arc::new(AtomicI32::new(0)),
        }
    }

    async fn fetch(&self) -> Result<String> {
        let attempt = self.attempt.fetch_add(1, Ordering::SeqCst);
        
        // Simulate failures for the first 3 attempts
        if attempt < 3 {
            return Err(anyhow::anyhow!("Service temporarily unavailable"));
        }
        
        Ok("Success!".to_string())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize a budget with 10 retry tokens
    let budget = RetryBudget::new(10);
    let service = UnreliableService::new();
    
    println!("Initial budget balance: {}", budget.balance());
    
    // Create a closure that captures both the budget and service
    let budget_clone = budget.clone();
    let fetch_with_budget = || {
        let budget = budget_clone.clone();
        let service_ref = &service;
        async move {
            service_ref.fetch().await
        }
    };

    // Use backon's retry mechanism with budget control
    let result = fetch_with_budget
        .retry(ExponentialBuilder::default().with_max_times(10))
        .when(|_error| {
            // Only retry if we have budget available
            if budget.withdraw() {
                println!("Retry attempted, budget remaining: {}", budget.balance());
                true
            } else {
                println!("Retry budget exhausted!");
                false
            }
        })
        .notify(|error, _duration| {
            println!("Operation failed: {}", error);
        })
        .await;

    match result {
        Ok(value) => {
            // Deposit back to budget on success
            budget.deposit();
            println!("Success! Result: {}", value);
            println!("Final budget balance: {}", budget.balance());
        }
        Err(e) => {
            println!("Failed after exhausting retries or budget: {}", e);
            println!("Final budget balance: {}", budget.balance());
        }
    }

    // Demonstrate budget recovery over time
    println!("\n--- Simulating time-based budget recovery ---");
    
    // In a real system, you might periodically replenish the budget
    for _ in 0..5 {
        budget.deposit();
    }
    println!("Budget after recovery: {}", budget.balance());

    Ok(())
}
```

## Key Concepts

1. **Budget Management**: The `RetryBudget` struct maintains an atomic counter that tracks available retry attempts.

2. **Withdraw/Deposit Pattern**: 
   - Withdrawals happen before each retry attempt
   - Deposits happen after successful operations
   - This creates a self-regulating system that prevents retry storms

3. **Integration with backon**: The example uses the `.when()` method to check budget availability before each retry.

4. **Thread-Safe**: Uses atomic operations to ensure the budget can be shared across async tasks.

## Benefits

- **Prevents Retry Storms**: When a service is down, the budget limits the total number of retries across all operations.
- **Self-Healing**: Successful operations restore budget, allowing the system to recover.
- **Configurable**: You can adjust initial balance, withdraw amounts, and deposit amounts based on your needs.

## Advanced Usage

In production systems, you might enhance this pattern with:
- Time-based budget recovery (e.g., gradually restore budget over time)
- Different withdraw costs for different error types
- Per-endpoint or per-service budget pools
- Integration with circuit breakers for additional protection