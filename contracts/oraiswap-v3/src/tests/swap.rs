use decimal::Factories;

use crate::{percentage::Percentage, FeeTier};

use super::helper::TestTubeScenario;

#[test]
fn test_swap_x_to_y() {
    let protocol_fee = Percentage::from_scale(6, 3);
    let mut scenario = TestTubeScenario::new(protocol_fee);

    let fee_tier = FeeTier::new(protocol_fee, 10).unwrap();

    let res = scenario.add_fee_tier(1, fee_tier).unwrap_err();

    println!("{:?}", res);
}
