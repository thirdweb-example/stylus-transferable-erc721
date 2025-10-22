#![cfg_attr(not(any(feature = "export-abi", test)), no_main)]
extern crate alloc;

use alloc::vec::Vec;
use alloy_primitives::{Address, FixedBytes, U256};
use alloy_sol_types::sol;
use stylus_sdk::{
    abi::Bytes,
    host::VM,
    prelude::*,
    storage::{StorageBool, StorageMap},
};

// ERC-7201 storage slot for "token.transferable"
// Calculated as: keccak256(abi.encode(uint256(keccak256("token.transferable")) - 1)) & ~bytes32(uint256(0xff))
const TRANSFERABLE_STORAGE_POSITION: U256 = U256::from_be_bytes([
    0x32, 0x4c, 0x74, 0xba, 0x20, 0x97, 0x60, 0x24, 0x4d, 0x63, 0x14, 0x3f, 0xd3, 0x3a, 0xf4, 0x2e,
    0x11, 0x68, 0x1d, 0x6b, 0x19, 0x8f, 0x7e, 0x7e, 0x1b, 0x6e, 0xf3, 0xeb, 0x45, 0x97, 0x6d, 0x00
]);

pub const ERROR_TRANSFER_DISABLED: u8 = 1;

sol! {
    error TransferableError(uint8 code);
}

#[derive(SolidityError)]
pub enum TransferableErrors {
    TransferableError(TransferableError),
}

pub struct CallbackFunction {
    pub selector: FixedBytes<4>,
}

pub struct FallbackFunction {
    pub selector: FixedBytes<4>,
    pub permission_bits: U256,
}

pub struct ModuleConfig {
    pub callback_functions: Vec<CallbackFunction>,
    pub fallback_functions: Vec<FallbackFunction>,
    pub required_interfaces: Vec<FixedBytes<4>>,
    pub register_installation_callback: bool,
}

struct TransferableStorage {
    transfer_enabled: StorageBool,
    transfer_enabled_for: StorageMap<Address, StorageBool>,
}

impl TransferableStorage {
    fn load(vm: &VM) -> Self {
        unsafe {
            Self {
                transfer_enabled: StorageBool::new(TRANSFERABLE_STORAGE_POSITION, 0, vm.clone()),
                transfer_enabled_for: StorageMap::new(TRANSFERABLE_STORAGE_POSITION + U256::from(1), 0, vm.clone()),
            }
        }
    }
}

sol_storage! {
    #[entrypoint]
    pub struct StylusTransferableERC721 {
    }
}

#[public]
impl StylusTransferableERC721 {

    pub fn get_module_config(&self) -> (bool, Vec<FixedBytes<4>>, Vec<FixedBytes<4>>, Vec<FixedBytes<4>>, Vec<(FixedBytes<4>, U256)>) {
        let register_installation_callback = false;
        
        let required_interfaces = vec![
            FixedBytes::from([0x80, 0xac, 0x58, 0xcd]), // ERC721 interface
        ];
        
        let supported_interfaces = vec![];
        
        let callback_functions = vec![
            FixedBytes::from([0x31, 0x41, 0x59, 0x26]), // beforeTransferERC721 selector
        ];
        
        let fallback_functions = vec![
            (FixedBytes::from([0x13, 0x57, 0x9b, 0xd7]), U256::ZERO), // isTransferEnabled, no permission
            (FixedBytes::from([0x29, 0x4c, 0x6f, 0x8a]), U256::ZERO), // isTransferEnabledFor, no permission
            (FixedBytes::from([0x4a, 0x6e, 0x1b, 0x1c]), U256::from(2)), // setTransferable, _MANAGER_ROLE
            (FixedBytes::from([0x5b, 0x7f, 0x2c, 0x3d]), U256::from(2)), // setTransferableFor, _MANAGER_ROLE
        ];
        
        (register_installation_callback, required_interfaces, supported_interfaces, callback_functions, fallback_functions)
    }

    pub fn before_transfer_erc721(
        &mut self,
        from: Address,
        to: Address,
        _token_id: U256
    ) -> Result<Bytes, TransferableErrors> {
        let storage = TransferableStorage::load(&self.vm());
        
        let is_operator_allowed = 
            storage.transfer_enabled_for.get(self.vm().msg_sender()) ||
            storage.transfer_enabled_for.get(from) ||
            storage.transfer_enabled_for.get(to);

        if !is_operator_allowed && !storage.transfer_enabled.get() {
            return Err(TransferableErrors::TransferableError(TransferableError { code: ERROR_TRANSFER_DISABLED }));
        }
        
        Ok(Bytes(vec![].into()))
    }

    pub fn is_transfer_enabled(&self) -> bool {
        TransferableStorage::load(&self.vm()).transfer_enabled.get()
    }

    pub fn is_transfer_enabled_for(&self, target: Address) -> bool {
        TransferableStorage::load(&self.vm()).transfer_enabled_for.get(target)
    }

    pub fn set_transferable(&mut self, enable_transfer: bool) -> Result<(), TransferableErrors> {
        TransferableStorage::load(&self.vm()).transfer_enabled.set(enable_transfer);
        Ok(())
    }

    pub fn set_transferable_for(&mut self, target: Address, enable_transfer: bool) -> Result<(), TransferableErrors> {
        TransferableStorage::load(&self.vm()).transfer_enabled_for.insert(target, enable_transfer);
        Ok(())
    }
}