use cosmwasm_std::{
    entry_point, to_binary,   CosmosMsg, Deps, DepsMut,Binary,
    Env, MessageInfo,  Response, StdResult, Uint128, WasmMsg,  Order
};

use cw2::set_contract_version;
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{
    State,CONFIG,TOKENINFO,OWNEDTOKEN, TokenInfo
};
use cw721::{Cw721ExecuteMsg, Cw721ReceiveMsg,Cw721QueryMsg, AllNftInfoResponse};
use cw20::{Cw20ExecuteMsg};


const CONTRACT_NAME: &str = "NFT_STAKING";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
     set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let state = State {
        owner:info.sender.to_string(),
        denom:msg.denom,
        staking_period : msg.staking_period,
        distribute_period:msg.distribute_period,
        reward_wallet : msg.reward_wallet,
        total_staked : Uint128::new(0),
        nft_address : msg.nft_address,
        token_address : msg.token_address,
        can_stake: true,
        last_distribute:env.block.time.seconds(),
        claim_reward:msg.claim_reward
    };
    CONFIG.save(deps.storage,&state)?;
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ReceiveNft(rcv_msg) => execute_stake_nft(deps, env, info, rcv_msg),
        ExecuteMsg::UnstakeNft { token_id } => execute_unstake_nft(deps, env, info, token_id),
        ExecuteMsg::WithdrawNft { token_id } => execute_withdraw_nft(deps, env, info, token_id),
        ExecuteMsg::GetReward {token_ids} =>execute_get_reward(deps, env, info, token_ids),
        ExecuteMsg::DistributeReward { token_amount } => execute_distribute_reward(deps,env,info,token_amount),
        ExecuteMsg::SetRewardWallet { address } => execute_reward_wallet(deps,env,info,address),
        ExecuteMsg::SetTokenAddress { address } => execute_token_address(deps,env,info,address),
        ExecuteMsg::SetOwner { address } => execute_set_owner(deps, env, info, address),
        ExecuteMsg::SetStakingPeriod { time } => execute_staking_period(deps,env,info,time),
        ExecuteMsg::SetStake { flag } => execute_set_stake(deps,info,flag),
        ExecuteMsg::SetDistributePeriod { time } => execute_distribute_period(deps, env, info, time),
        ExecuteMsg::Migrate {amount,address,id} => execute_migrate_token(deps, env, info, amount,address,id),
        ExecuteMsg::SetClaimAmount { amount }=> execute_claim_amount(deps, env, info, amount),
        ExecuteMsg::AddNftAddress { address } => execute_nft_address(deps,env,info,address)
    }
}

fn execute_stake_nft(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    rcv_msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    let state = CONFIG.load(deps.storage)?;

    let token = TOKENINFO.may_load(deps.storage, &rcv_msg.token_id.clone())?;

    if state.can_stake == false{
        return Err(ContractError::CanNotStake{})
    }
    
    let sender = info.sender.to_string();
    let mut is_registered = false;

    for address in state.nft_address{
        if address == sender{
            is_registered = true
        }
    }

    if !is_registered{
        return Err(ContractError::WrongNftContract {  });
    } 

    if token != None {
        return Err(ContractError::AlreadyStaked {  });
    }
   
    CONFIG.update(deps.storage,
        |mut state|->StdResult<_>{
            state.total_staked = state.total_staked + Uint128::new(1) ;
            Ok(state)
        }
    )?;

    let token_info = TokenInfo{
        owner:rcv_msg.sender.clone(),
        token_id:rcv_msg.token_id.clone(),
        status : "Staked".to_string(),
        unstake_time : 0,
        stake_time :env.block.time.seconds(),
        reward: Uint128::new(0),
        nft_address:sender
    };

    let my_nfts = OWNEDTOKEN.may_load(deps.storage,&rcv_msg.sender.clone().to_string())?;

    if my_nfts == None{
        let mut token_ids:Vec<String> = vec![];
        token_ids.push(rcv_msg.token_id.clone());
        OWNEDTOKEN.save(deps.storage,&rcv_msg.sender,&token_ids)?;

    }

    else{
        let mut token_ids = my_nfts.unwrap();
        token_ids.push(rcv_msg.token_id.clone());
        OWNEDTOKEN.update(deps.storage,&rcv_msg.sender,
        |_my_nfts|->StdResult<_>{
            Ok(token_ids)
        }
    )?;
    }

    TOKENINFO.save(deps.storage, &rcv_msg.token_id.clone(), &token_info)?;
    
    Ok(Response::default())

}



fn execute_unstake_nft(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    
) -> Result<Response, ContractError> {
    // let state = CONFIG.load(deps.storage)?;

    let token = TOKENINFO.may_load(deps.storage, &token_id)?;

    if token == None {
        return Err(ContractError::NotStaked {  });
    }
   
   else {
      let  token = token.unwrap();    

      if token.owner != info.sender.to_string(){
          return Err(ContractError::Unauthorized {  })
      }

      TOKENINFO.update(deps.storage,&token_id,
        |token_info|->StdResult<_>{
            let mut token_info = token_info.unwrap();
            token_info.status = "Unstaking".to_string();
            token_info.unstake_time = env.block.time.seconds();
            
            Ok(token_info)
        }
    )?;
   }
     CONFIG.update(deps.storage,
        |mut state|->StdResult<_>{
            state.total_staked = state.total_staked-Uint128::new(1);
            Ok(state)
        })?;

    
    Ok(Response::default())

}

fn execute_withdraw_nft(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    
) -> Result<Response, ContractError> {
    let state = CONFIG.load(deps.storage)?;
    let mut nft_address:String ;

    let token = TOKENINFO.may_load(deps.storage, &token_id)?;

    let mut messages:Vec<CosmosMsg> = vec![];
    
    if token == None {
        return Err(ContractError::NotStaked {  });
    }
   
   else {
      let  token = token.unwrap();

       if token.owner != info.sender.to_string(){
          return Err(ContractError::Unauthorized {  })
      }

      if token.status =="Staked".to_string(){
          return Err(ContractError::StatusError {  })
      }

      if (env.block.time.seconds() - token.unstake_time)<state.staking_period{
           return Err(ContractError::TimeRemaining {  })
      
        }

      nft_address = token.nft_address; 
       
      if token.reward > Uint128::new(0){
      messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
             contract_addr: state.token_address.clone(), 
             msg: to_binary(&Cw20ExecuteMsg::Transfer {
                 recipient: token.owner, 
                 amount: token.reward 
                })? , 
        funds: vec![] }));
    }
    
    TOKENINFO.remove(deps.storage,&token_id);
    
   }

   let my_nfts = OWNEDTOKEN.load(deps.storage,&info.sender.to_string())?;
   let mut new_nfts:Vec<String> = vec![];
   for id  in my_nfts{
     if id !=  token_id{
         new_nfts.push(id)
     }
   }   

   OWNEDTOKEN.save(deps.storage,&info.sender.to_string(),&new_nfts)?;
  
   Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
             contract_addr: nft_address, 
             msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                  recipient: info.sender.to_string(), 
                  token_id: token_id })? , 
             funds: vec![] }))
        .add_messages(messages)
         
)
}



fn execute_get_reward(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token_ids: Vec<String>,
    
) -> Result<Response, ContractError> {
    let state = CONFIG.load(deps.storage)?;

    let mut messages:Vec<CosmosMsg> = vec![];

    for token_id in token_ids{
    let token = TOKENINFO.may_load(deps.storage, &token_id)?;
    if token == None {
        return Err(ContractError::NotStaked {  });
    }
   
   else {
      let  token = token.unwrap();

       if token.owner != info.sender.to_string(){
          return Err(ContractError::Unauthorized {  })
      }

       
      if token.reward > Uint128::new(0){
         messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
             contract_addr: state.token_address.clone(), 
             msg: to_binary(&Cw20ExecuteMsg::Transfer {
                 recipient: token.owner, 
                 amount: token.reward 
                })? , 
             funds: vec![] }));
        }

      TOKENINFO.update(deps.storage,&token_id,
        |token_info|->StdResult<_>{
            let mut token_info = token_info.unwrap();
            token_info.reward = Uint128::new(0);
            Ok(token_info)
        })?;
   }
}
   
   Ok(Response::new()
        .add_messages(messages)
         
)
}






fn execute_distribute_reward(
    deps: DepsMut,
    env:  Env,
    info: MessageInfo,
    token_amount:Uint128
)->Result<Response,ContractError>{

    let state = CONFIG.load(deps.storage)?;

    if info.sender.to_string() != state.reward_wallet{
        return Err(ContractError::Unauthorized {});
    }

    if (env.block.time.seconds() - state.last_distribute)<state.distribute_period{
        return Err(ContractError::CanNotDistribute {  })
    }
    
    let token_id :StdResult<Vec<String>>  = TOKENINFO
        .keys(deps.storage, None, None, Order::Ascending)
        .collect();

    let token_group = token_id?;

   if token_group.len() == 0 {
       return Err(ContractError::NotStaked {  })
   }

    if state.total_staked == Uint128::new(0){
        return Err(ContractError::NotStaked {  })
    }
   

    for token_id in token_group{
            let token_info = TOKENINFO.load(deps.storage,&token_id)?;
            if token_info.status == "Staked".to_string()
            {       TOKENINFO.update(deps.storage, &token_id,
                |token_info|->StdResult<_>{
                    let mut token_info = token_info.unwrap();
                    token_info.reward = token_info.reward + token_amount/state.total_staked;
                    Ok(token_info)
            }
            )?; }
    }

    CONFIG.update(deps.storage,
        |mut state|-> StdResult<_>{
            state.last_distribute = env.block.time.seconds();
            Ok(state)
        }    
    )?;

    Ok(Response::default())
}



fn execute_reward_wallet(
    deps: DepsMut,
    _env : Env,
    info: MessageInfo,
    address: String,
)->Result<Response,ContractError>{

    let state = CONFIG.load(deps.storage)?;

    if info.sender.to_string() != state.owner{
        return Err(ContractError::Unauthorized {});
    }
    CONFIG.update(deps.storage,
    |mut state|->StdResult<_>{
        state.reward_wallet = address;
        Ok(state)
    })?;
    Ok(Response::default())
}


fn execute_nft_address(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let state = CONFIG.load(deps.storage)?;
    deps.api.addr_validate(&address)?;
   
    if state.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }

    CONFIG.update(deps.storage, 
        |mut state| -> StdResult<_>{
            state.nft_address.push(address);
            Ok(state)
        } )?;

    Ok(Response::default())
}

fn execute_token_address(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let mut state = CONFIG.load(deps.storage)?;
    deps.api.addr_validate(&address)?;
    state.token_address = address;
    
    if state.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }

    CONFIG.save(deps.storage, &state)?;
    Ok(Response::default())
}


fn execute_set_owner(
    deps: DepsMut,
     _env:Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let mut state = CONFIG.load(deps.storage)?;
    deps.api.addr_validate(&address)?;
    state.owner = address;
    
    if state.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }

    CONFIG.save(deps.storage, &state)?;
    Ok(Response::default())
}



fn execute_staking_period(
    deps: DepsMut,
    _env : Env,
    info: MessageInfo,
    time: u64,
)->Result<Response,ContractError>{

    let state = CONFIG.load(deps.storage)?;

    if info.sender.to_string() != state.owner{
        return Err(ContractError::Unauthorized {});
    }
    CONFIG.update(deps.storage,
    |mut state|->StdResult<_>{
        state.staking_period = time;
        Ok(state)
    })?;
    Ok(Response::default())
}


fn execute_claim_amount(
    deps: DepsMut,
    _env : Env,
    info: MessageInfo,
    amount: Uint128,
)->Result<Response,ContractError>{

    let state = CONFIG.load(deps.storage)?;

    if info.sender.to_string() != state.owner{
        return Err(ContractError::Unauthorized {});
    }
    CONFIG.update(deps.storage,
    |mut state|->StdResult<_>{
        state.claim_reward = amount;
        Ok(state)
    })?;
    Ok(Response::default())
}



fn execute_distribute_period(
    deps: DepsMut,
    _env : Env,
    info: MessageInfo,
    time: u64,
)->Result<Response,ContractError>{

    let state = CONFIG.load(deps.storage)?;

    if info.sender.to_string() != state.owner{
        return Err(ContractError::Unauthorized {});
    }
    CONFIG.update(deps.storage,
    |mut state|->StdResult<_>{
        state.distribute_period = time;
        Ok(state)
    })?;
    Ok(Response::default())
}



fn execute_set_stake(
    deps: DepsMut,

    info: MessageInfo,
    flag: bool,
)->Result<Response,ContractError>{

    let state = CONFIG.load(deps.storage)?;

    if info.sender.to_string() != state.owner{
        return Err(ContractError::Unauthorized {});
    }
    CONFIG.update(deps.storage,
    |mut state|->StdResult<_>{
        state.can_stake = flag;
        Ok(state)
    })?;
    Ok(Response::default())
}





fn execute_migrate_token(
    deps: DepsMut,
    _env : Env,
    info: MessageInfo,
    amount: Uint128,
    address:String,
    id:Vec<String>
)->Result<Response,ContractError>{

    deps.api.addr_validate(&address)?;
    let state = CONFIG.load(deps.storage)?;

    if info.sender.to_string() != state.owner{
        return Err(ContractError::Unauthorized {});
    }

    let mut messages :Vec<CosmosMsg> = Vec::new();


    for token_id in id{
        let token_info = TOKENINFO.may_load(deps.storage, &token_id)?;
        if token_info != None{
            let token_info = token_info.unwrap();
            messages.push(
                CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_info.nft_address, 
                msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: address.clone(), 
                    token_id: token_id 
                    })? , 
                funds: vec![] })
            )
        }
    }
   
    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
             contract_addr: state.token_address, 
             msg: to_binary(&Cw20ExecuteMsg::Transfer {
                 recipient: address, 
                 amount: amount 
                })? , 
             funds: vec![] }))
        .add_messages(messages)
)
}






#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
          QueryMsg::GetStateInfo {  } => to_binary(&query_state_info(deps)?),
          QueryMsg::GetCurrentTime{} => to_binary(&query_get_current_time(deps,_env)?),
          QueryMsg::GetToken { token_id } => to_binary(&query_get_token(deps,token_id)?),
          QueryMsg::GetMyIds { address } => to_binary(&query_my_ids(deps,address)?),
          QueryMsg::GetMyInfo { address }=> to_binary(&query_my_info(deps,address)?),
  }
}

pub fn query_state_info(deps:Deps) -> StdResult<State>{
    let state =  CONFIG.load(deps.storage)?;
    Ok(state)
}

pub fn query_get_current_time(_deps:Deps,env:Env) -> StdResult<u64>{
    Ok(env.block.time.seconds())
}

pub fn query_get_members(deps:Deps) -> StdResult<Vec<String>>{
     let token_id :StdResult<Vec<String>>  = TOKENINFO
        .keys(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(token_id?)
}

pub fn query_token_info(deps:Deps)->StdResult<Vec<TokenInfo>>{
      let res: StdResult<Vec<TokenInfo>> = TOKENINFO
        .range(deps.storage, None, None, Order::Ascending)
        .map(|kv_item|parse_token_info(kv_item))
        .collect();
    Ok(res?)
}


fn parse_token_info(
    item: StdResult<(String,TokenInfo)>,
) -> StdResult<TokenInfo> {
    item.and_then(|(_k, token_info)| {
        Ok(token_info)
    })
}


pub fn query_get_token(deps:Deps,token_id:String) -> StdResult<TokenInfo>{
    let token_info = TOKENINFO.load(deps.storage,&token_id)?;
    Ok(token_info)
}

pub fn query_my_ids(deps:Deps,address:String) -> StdResult<Vec<String>>{
    let my_ids = OWNEDTOKEN.may_load(deps.storage,&address)?;
    if my_ids == None{
        Ok(vec![])
    }
    else{
        Ok(my_ids.unwrap())
    }
}

pub fn query_my_info(deps:Deps,address:String) -> StdResult<Vec<TokenInfo>>{
    let my_ids = OWNEDTOKEN.may_load(deps.storage,&address)?;
    if my_ids == None{
        Ok(vec![])
    }
    else{
        let mut my_nfts:Vec<TokenInfo> = vec![];
        for id in my_ids.unwrap(){            
            let token_info = TOKENINFO.load(deps.storage, &id)?;
            my_nfts.push(token_info);         
        }
          Ok(my_nfts)
    }
}


#[cfg(test)]
mod tests {

    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{CosmosMsg};

    #[test]
    fn testing() {
        let mut deps = mock_dependencies();
        let instantiate_msg = InstantiateMsg {
            denom : "ujuno".to_string(),
            staking_period : 1000,
            reward_wallet :"reward_wallet".to_string(),
            distribute_period:100,
            token_address:"token_address".to_string(),
            nft_address:vec!["nft_address".to_string()],
            claim_reward:Uint128::new(500)
        };
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());


        let state = query_state_info(deps.as_ref()).unwrap();
        assert_eq!(state,State{
            nft_address :vec!["nft_address".to_string()],
            token_address : "token_address".to_string(),
            owner:"creator".to_string(),
            staking_period : 1000,
            denom : "ujuno".to_string(),
            reward_wallet:"reward_wallet".to_string(),
            total_staked:Uint128::new(0),
            can_stake : true,
            last_distribute : mock_env().block.time.seconds(),
            distribute_period:100,
            claim_reward:Uint128::new(500)
        });

        println!("{:?}","add nft address");

        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::AddNftAddress { address:"nft_address1".to_string() };
        execute(deps.as_mut(),mock_env(),info,msg).unwrap();

        println!("{:?}","add token address");

        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::SetTokenAddress { address:"token_address1".to_string() };
        execute(deps.as_mut(),mock_env(),info,msg).unwrap();

        let state = query_state_info(deps.as_ref()).unwrap();
        assert_eq!(state.nft_address,vec!["nft_address".to_string(),"nft_address1".to_string()]);
        assert_eq!(state.token_address, "token_address1".to_string());

        println!("{:?}","set reward wallet");

        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::SetRewardWallet { address:"reward_wallet1".to_string() };
        execute(deps.as_mut(),mock_env(),info,msg).unwrap();

         println!("{:?}","set distribute period");


        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::SetDistributePeriod { time:150 };
        execute(deps.as_mut(),mock_env(),info,msg).unwrap();

        let state= query_state_info(deps.as_ref()).unwrap();
        assert_eq!(state.distribute_period,150);

        let state = query_state_info(deps.as_ref()).unwrap();
        assert_eq!(state.reward_wallet,"reward_wallet1".to_string());
       
         println!("{:?}","set staking period");


        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::SetStakingPeriod { time: 1200 };
        execute(deps.as_mut(),mock_env(),info,msg).unwrap();
        
        let state = query_state_info(deps.as_ref()).unwrap();
        assert_eq!(state.staking_period,1200);

        let info = mock_info("nft_address", &[]);
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
            sender:"owner1".to_string(),
            token_id : "reveal1".to_string(),
            msg : to_binary(&"abc".to_string()).unwrap()
        });
        execute(deps.as_mut(),mock_env(),info,msg).unwrap();

        let info = mock_info("nft_address1", &[]);
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
            sender:"owner1".to_string(),
            token_id : "reveal2".to_string(),
            msg : to_binary(&"abc".to_string()).unwrap()
        });
        execute(deps.as_mut(),mock_env(),info,msg).unwrap();

        // let my_nfs = query_my_ids(deps,"")

        let tokens = query_get_members(deps.as_ref()).unwrap();
        assert_eq!(tokens,vec!["reveal1","reveal2"]);

        let my_ids = query_my_ids(deps.as_ref(), "owner2".to_string()).unwrap();
        let eq_my_ids:Vec<String> = vec![];
        assert_eq!(my_ids,eq_my_ids);

        println!("{:?}","tokens of owner1");

        let my_ids = query_my_ids(deps.as_ref(), "owner1".to_string()).unwrap();
        assert_eq!(my_ids,["reveal1","reveal2"]);

        println!("{:?}","tokens of owner2");

        let my_token_infos = query_my_info(deps.as_ref(),"owner2".to_string()).unwrap();
        let eq_my_ids:Vec<TokenInfo> = vec![];
        assert_eq!(my_token_infos,eq_my_ids);

        println!("{:?}","token informations of owner1");

        let my_token_infos = query_my_info(deps.as_ref(),"owner1".to_string()).unwrap();
        assert_eq!(my_token_infos,vec![TokenInfo{
            owner:"owner1".to_string(),
            token_id:"reveal1".to_string(),
            stake_time:mock_env().block.time.seconds(),
            status:"Staked".to_string(),
            reward:Uint128::new(0),
            unstake_time :0,
            nft_address:"nft_address".to_string()
        },TokenInfo{
            owner:"owner1".to_string(),
            token_id:"reveal2".to_string(),
            stake_time:mock_env().block.time.seconds(),
            status:"Staked".to_string(),
            reward:Uint128::new(0),
            unstake_time :0,
            nft_address:"nft_address1".to_string()
        }]);
      
       println!("{:?}","unstake reveal1");


        let info = mock_info("owner1", &[]);
        let msg = ExecuteMsg::UnstakeNft { token_id : "reveal1".to_string() };
        execute(deps.as_mut(),mock_env(),info,msg).unwrap();

        let state =  query_state_info(deps.as_ref()).unwrap();

        assert_eq!(state.total_staked,Uint128::new(1));

        let tokens = query_get_members(deps.as_ref()).unwrap();
        assert_eq!(tokens,vec!["reveal1","reveal2"]);

        let token_infos = query_token_info(deps.as_ref()).unwrap();
        assert_eq!(token_infos,vec![TokenInfo{
            owner:"owner1".to_string(),
            token_id:"reveal1".to_string(),
            stake_time:mock_env().block.time.seconds(),
            status:"Unstaking".to_string(),
            reward:Uint128::new(0),
            unstake_time : mock_env().block.time.seconds(),
             nft_address:"nft_address".to_string()
        },TokenInfo{
            owner:"owner1".to_string(),
            token_id:"reveal2".to_string(),
            stake_time:mock_env().block.time.seconds(),
            status:"Staked".to_string(),
            reward:Uint128::new(0),
            unstake_time :0,
             nft_address:"nft_address1".to_string()
        }]);

        let my_token_infos = query_my_info(deps.as_ref(),"owner1".to_string()).unwrap();
        assert_eq!(my_token_infos,vec![TokenInfo{
            owner:"owner1".to_string(),
            token_id:"reveal1".to_string(),
            stake_time:mock_env().block.time.seconds(),
            status:"Unstaking".to_string(),
            reward:Uint128::new(0),
            unstake_time : mock_env().block.time.seconds(),
             nft_address:"nft_address".to_string()
        },TokenInfo{
            owner:"owner1".to_string(),
            token_id:"reveal2".to_string(),
            stake_time:mock_env().block.time.seconds(),
            status:"Staked".to_string(),
            reward:Uint128::new(0),
            unstake_time :0,
             nft_address:"nft_address1".to_string()
        }]);


        let info = mock_info("reward_wallet1", &[]);     
        let msg = ExecuteMsg::DistributeReward { token_amount:Uint128::new(10)  };
        execute(deps.as_mut(),mock_env(),info,msg).unwrap();

        println!("{:?}","check the reward distribution");

        let token_infos = query_token_info(deps.as_ref()).unwrap();
        assert_eq!(token_infos,vec![TokenInfo{
            owner:"owner1".to_string(),
            token_id:"reveal1".to_string(),
            stake_time:mock_env().block.time.seconds(),
            status:"Unstaking".to_string(),
            reward:Uint128::new(0),
            unstake_time : mock_env().block.time.seconds(),
            nft_address:"nft_address".to_string()
        },TokenInfo{
            owner:"owner1".to_string(),
            token_id:"reveal2".to_string(),
            stake_time:mock_env().block.time.seconds(),
            status:"Staked".to_string(),
            reward:Uint128::new(10),
            unstake_time :0,
            nft_address:"nft_address1".to_string()
        }]);

        let info = mock_info("owner1", &[]);     
        let msg = ExecuteMsg::GetReward { token_ids:vec!["reveal1".to_string(),"reveal2".to_string()] };
        let res = execute(deps.as_mut(),mock_env(),info,msg).unwrap();
        assert_eq!(1,res.messages.len());
        assert_eq!(res.messages[0].msg,CosmosMsg::Wasm(WasmMsg::Execute {
             contract_addr: "token_address1".to_string(), 
             msg: to_binary(&Cw20ExecuteMsg::Transfer {
                 recipient: "owner1".to_string(), 
                 amount: Uint128::new(10) 
                }).unwrap() , 
             funds: vec![] }));

       
        
        let info = mock_info("owner1", &[]);     
        let msg = ExecuteMsg::WithdrawNft { token_id:"reveal1".to_string() };
        let res = execute(deps.as_mut(),mock_env(),info,msg).unwrap();
        
        
        let my_ids = query_my_ids(deps.as_ref(), "owner1".to_string()).unwrap();
        assert_eq!(my_ids,["reveal2"]);

        assert_eq!(1,res.messages.len());
        assert_eq!(res.messages[0].msg,CosmosMsg::Wasm(WasmMsg::Execute {
             contract_addr: "nft_address".to_string(), 
             msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                  recipient: "owner1".to_string(), 
                  token_id: "reveal1".to_string() }).unwrap() , 
             funds: vec![] }));

        let tokens = query_get_members(deps.as_ref()).unwrap();
        assert_eq!(tokens,vec!["reveal2"]);

        let id_info = query_get_token(deps.as_ref(),"reveal2".to_string()).unwrap();
        assert_eq!(id_info,TokenInfo{
            owner:"owner1".to_string(),
            token_id:"reveal2".to_string(),
            stake_time:mock_env().block.time.seconds(),
            status:"Staked".to_string(),
            reward:Uint128::new(0),
            unstake_time :0,
            nft_address:"nft_address1".to_string()
        });

      let info = mock_info("creator", &[]);     
      let msg = ExecuteMsg::Migrate { amount: Uint128::new(10), address: "new_staking".to_string(), id: vec!["reveal1".to_string(),"reveal2".to_string()] };
      let res = execute(deps.as_mut(),mock_env(),info,msg).unwrap();
      assert_eq!(res.messages.len(),2);
      
      assert_eq!(res.messages[1].msg,CosmosMsg::Wasm(WasmMsg::Execute {
             contract_addr: "nft_address1".to_string(), 
             msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                  recipient: "new_staking".to_string(), 
                  token_id: "reveal2".to_string() }).unwrap() , 
             funds: vec![] }));

       assert_eq!(res.messages[0].msg,CosmosMsg::Wasm(WasmMsg::Execute {
             contract_addr: "token_address1".to_string(), 
             msg: to_binary(&Cw20ExecuteMsg::Transfer {
                 recipient: "new_staking".to_string(), 
                 amount: Uint128::new(10) 
                }).unwrap() , 
        funds: vec![] }));

    
    }
}
