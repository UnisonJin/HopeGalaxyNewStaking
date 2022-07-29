use cosmwasm_std::{ Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw721::Cw721ReceiveMsg;



#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub denom:String,
    pub staking_period : u64,
    pub reward_wallet : String,
    pub distribute_period: u64,
    pub nft_address:Vec<String>,
    pub token_address:String,
    pub claim_reward:Uint128
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ReceiveNft(Cw721ReceiveMsg),
    UnstakeNft{token_id:String},
    WithdrawNft{token_id:String},
    GetReward{token_ids:Vec<String>},  
    DistributeReward{token_amount:Uint128},
    SetRewardWallet{address:String},
    SetOwner{address:String},
    SetStakingPeriod{time:u64},   
    SetTokenAddress{address:String},
    SetStake{flag:bool},
    SetDistributePeriod{time:u64},
    Migrate{amount:Uint128,address:String,id:Vec<String>},
    SetClaimAmount{amount:Uint128},
    AddNftAddress{address:String}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
  GetStateInfo{},
  GetCurrentTime{},
  GetToken{token_id:String},
  GetMyIds{address:String},
  GetMyInfo{address:String},
}

