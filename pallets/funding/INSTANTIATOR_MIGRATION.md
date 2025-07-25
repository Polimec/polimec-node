# Instantiator Migration Guide

This document explains the improvements made to the funding pallet's instantiator and how to migrate existing tests.

## Overview

The instantiator has been significantly improved to address several key issues:

### Problems with the Old Instantiator

1. **Logic Divergence**: Custom calculations that didn't match the pallet's actual logic
2. **Complex Setup**: Methods like `create_finished_project` trying to do too much at once
3. **Hard to Debug**: Complex calculation chains made it difficult to isolate issues
4. **Maintenance Burden**: Updates to pallet logic required corresponding updates to instantiator
5. **Calculation Mismatches**: Rounding differences and price calculation inconsistencies

### Benefits of the New Instantiator

1. **Pallet Alignment**: Uses actual pallet logic instead of reimplementing it
2. **Reliability**: Eliminates calculation mismatches between tests and pallet
3. **Simplicity**: Fewer lines of code, easier to understand
4. **Debugging**: Clear error messages, step-by-step validation
5. **Maintainability**: Changes to pallet automatically reflected in tests
6. **Composability**: Small, focused methods can be combined flexibly
7. **Validation**: Built-in validation catches issues early

## New Architecture

The improved instantiator is organized into several focused modules:

### Core Modules

```
src/instantiator/
├── mod.rs                    # Main module
├── types.rs                  # Existing types
├── traits.rs                 # Existing traits
├── pallet_integration.rs     # NEW: Pallet-aligned calculations
├── improved_flow.rs          # NEW: Step-by-step project creation
├── validation.rs             # NEW: Validation and error handling
├── test_helpers.rs           # NEW: Focused test helpers
├── migration_example.rs      # NEW: Migration examples
├── chain_interactions.rs     # UPDATED: Deprecated old methods
└── calculations.rs           # UPDATED: Added new methods
```

### Key New Methods

#### Pallet Integration (`pallet_integration.rs`)
- `calculate_plmc_bond_with_pallet()` - Uses pallet's actual PLMC calculation
- `calculate_funding_asset_with_pallet()` - Uses pallet's asset calculation
- `simulate_ct_amount_from_funding_asset()` - Simulates pallet's CT calculation
- `get_exact_funding_requirements_for_bids()` - Accurate funding requirements
- `perform_bid_with_pallet_logic()` - Uses pallet's do_bid function
- `validate_bid_parameters()` - Uses pallet's validation logic

#### Improved Flow (`improved_flow.rs`)
- `create_project_with_pallet()` - Create project using pallet logic
- `start_evaluation_with_pallet()` - Start evaluation with validation
- `perform_evaluations_with_pallet()` - Perform evaluations with exact requirements
- `perform_bids_with_pallet()` - Perform bids with pallet logic
- `create_complete_project_with_pallet()` - Full project lifecycle

#### Test Helpers (`test_helpers.rs`)
- `create_minimal_project()` - Simple project creation
- `create_project_in_evaluation()` - Project in evaluation phase
- `create_project_in_auction()` - Project in auction phase  
- `create_completed_project()` - Project with specified funding level
- `create_successful_project()` - Successfully funded project
- `create_failed_project()` - Failed project
- `create_settled_project()` - Fully settled project
- `run_full_project_lifecycle_test()` - Complete lifecycle with validation

#### Validation (`validation.rs`)
- `validate_project_metadata()` - Validate metadata using pallet rules
- `validate_evaluation_params()` - Validate evaluation parameters
- `validate_bid_parameters()` - Validate bid parameters
- `validate_complete_project_setup()` - Comprehensive project validation
- `assert_pallet_state_consistency()` - Ensure pallet state is consistent

## Migration Examples

### Simple Test Migration

**Before:**
```rust
#[test]
fn auction_round_completed() {
    let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
    let project_metadata = default_project_metadata(ISSUER_1);
    let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
    let bids = inst.generate_bids_from_total_ct_percent(project_metadata.clone(), 60, 10);
    let _project_id = inst.create_finished_project(project_metadata, ISSUER_1, None, evaluations, bids);
}
```

**After:**
```rust
#[test]
fn auction_round_completed() {
    let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
    let _project_id = inst.create_completed_project(ISSUER_1, 5, 10, 60);
}
```

### Step-by-Step Migration

**Before:**
```rust
let project_id = inst.create_auctioning_project(metadata, issuer, None, evaluations);
// Custom bid calculation and setup...
inst.bid_for_users(project_id, bids).unwrap();
assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::FundingSuccessful);
```

**After:**
```rust
let project_id = inst.create_project_in_auction(issuer, 5);
let bids = inst.create_realistic_bids(project_id, 10, 70);
inst.perform_bids_with_pallet(project_id, bids).unwrap();
inst.end_funding_with_pallet(project_id).unwrap();
inst.assert_project_state(project_id, ProjectStatus::FundingSuccessful);
```

### Full Validation Migration

**Before:**
```rust
// No validation, potential for silent failures
let project_id = inst.create_finished_project(metadata, issuer, None, evaluations, bids);
```

**After:**
```rust
// Comprehensive validation at every step
let project_id = inst.run_full_project_lifecycle_test(issuer, 5, 10, 70);
inst.assert_pallet_state_consistency(project_id);
```

## Migration Strategy

### Phase 1: Update Simple Tests
- Replace `create_finished_project` with `create_completed_project`
- Replace `create_auctioning_project` with `create_project_in_auction`
- Replace `generate_successful_evaluations` with `generate_successful_evaluations_with_pallet`

### Phase 2: Add Validation
- Use `run_full_project_lifecycle_test` for comprehensive testing
- Add `assert_pallet_state_consistency` checks
- Use validation methods to catch issues early

### Phase 3: Use Step-by-Step Methods
- Break down complex tests into clear steps
- Use `create_project_with_pallet`, `start_evaluation_with_pallet`, etc.
- Add `assert_project_state` checks between steps

### Phase 4: Clean Up
- Remove deprecated method calls
- Update any remaining custom calculations
- Remove old helper methods once migrated

## Common Patterns

### Testing Project Failure
```rust
let project_id = inst.create_project_in_auction(ISSUER_1, 5);
// No bids = failure
inst.end_funding_with_pallet(project_id).unwrap();
inst.assert_project_state(project_id, ProjectStatus::FundingFailed);
```

### Testing Settlement
```rust
let project_id = inst.create_successful_project(ISSUER_1);
inst.start_settlement_with_pallet(project_id).unwrap();
inst.settle_project_with_pallet(project_id, true);
```

### Custom Scenarios
```rust
let evaluators = inst.create_test_accounts(5, "EVAL");
inst.setup_test_accounts_with_funds(&evaluators, 1_000_000 * PLMC_UNIT, &[]);

let project_id = inst.create_minimal_project(ISSUER_1);
// ... custom logic using step-by-step methods
```

### Realistic Testing
```rust
let bids = inst.create_realistic_bids(project_id, 15, 75);
// This creates diverse bids with different:
// - Investor types (Retail, Professional, Institutional)
// - Participation modes (Classic with different multipliers, OTM)
// - Funding assets (USDT, USDC, DOT)
// - Bid sizes (Large, medium, small)
```

## Debugging Tips

### Use Validation Methods
```rust
// Validate before operations to catch issues early
inst.validate_project_metadata(&metadata).unwrap();
inst.validate_evaluation_params(project_id, &evaluation).unwrap();
```

### Check State Consistency
```rust
// Verify pallet state is consistent
inst.assert_pallet_state_consistency(project_id);
inst.validate_bucket_state(project_id).unwrap();
```

### Step-by-Step Debugging
```rust
// Break complex operations into steps
let project_id = inst.create_project_with_pallet(metadata, issuer, None);
inst.assert_project_state(project_id, ProjectStatus::Application);

inst.start_evaluation_with_pallet(project_id).unwrap();
inst.assert_project_state(project_id, ProjectStatus::EvaluationRound);
// ... continue step by step
```

## Backward Compatibility

The old methods are marked as deprecated but still work by delegating to the new implementations:

```rust
#[deprecated(since = "0.1.0", note = "Use create_project_with_pallet for better pallet alignment")]
pub fn create_new_project(...) -> ProjectId {
    self.create_project_with_pallet(project_metadata, issuer, maybe_did)
}
```

This allows gradual migration without breaking existing tests.

## Best Practices

1. **Use Helper Methods**: Start with high-level helpers like `create_completed_project`
2. **Add Validation**: Use validation methods to catch issues early
3. **Step-by-Step for Complex Tests**: Use step-by-step methods for better control
4. **Assert State**: Use `assert_project_state` between operations
5. **Check Consistency**: Use `assert_pallet_state_consistency` for robustness
6. **Realistic Scenarios**: Use `create_realistic_bids` for diverse testing

## Files to Update

When migrating tests, you'll primarily need to update:

- `src/tests/1_application.rs` - Project creation tests
- `src/tests/2_evaluation.rs` - Evaluation round tests  
- `src/tests/3_auction.rs` - Auction round tests
- `src/tests/4_funding_end.rs` - Funding completion tests
- `src/tests/5_settlement.rs` - Settlement tests
- `src/tests/6_ct_migration.rs` - Migration tests
- `src/tests/misc.rs` - Miscellaneous tests

## Questions?

If you encounter issues during migration:

1. Check the examples in `migration_example.rs`
2. Look at the improved tests in `3_auction_improved.rs`
3. Use validation methods to identify the specific issue
4. Break complex operations into step-by-step methods for better debugging

The new instantiator provides much better error messages and validation to help identify issues quickly.