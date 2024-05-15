use alloy_primitives::U256;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FeeGrowthOutside {
    pub fee_growth_outside0_x128: U256,
    pub fee_growth_outside1_x128: U256,
}

pub fn get_fee_growth_inside(
    lower: FeeGrowthOutside,
    upper: FeeGrowthOutside,
    tick_lower: i32,
    tick_upper: i32,
    tick_current: i32,
    fee_growth_global0_x128: U256,
    fee_growth_global1_x128: U256,
) -> (U256, U256) {
    let fee_growth_inside0_x128: U256;
    let fee_growth_inside1_x128: U256;
    if tick_current < tick_lower {
        fee_growth_inside0_x128 = lower.fee_growth_outside0_x128 - upper.fee_growth_outside0_x128;
        fee_growth_inside1_x128 = lower.fee_growth_outside1_x128 - upper.fee_growth_outside1_x128;
    } else if tick_current >= tick_upper {
        fee_growth_inside0_x128 = upper.fee_growth_outside0_x128 - lower.fee_growth_outside0_x128;
        fee_growth_inside1_x128 = upper.fee_growth_outside1_x128 - lower.fee_growth_outside1_x128;
    } else {
        fee_growth_inside0_x128 = fee_growth_global0_x128
            - lower.fee_growth_outside0_x128
            - upper.fee_growth_outside0_x128;
        fee_growth_inside1_x128 = fee_growth_global1_x128
            - lower.fee_growth_outside1_x128
            - upper.fee_growth_outside1_x128;
    }
    (fee_growth_inside0_x128, fee_growth_inside1_x128)
}
