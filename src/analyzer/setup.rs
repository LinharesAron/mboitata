use std::path::PathBuf;

use tokio::task::JoinHandle;

use crate::{
    allow_list::AllowList,
    analyzer::{
        event::Dispatcher,
        router::StageRegistry,
        stage::StageId,
        stages::{
            filter_stage::FilterStage, map_stage::MapStage, save_file_stage::SaveFileStage,
            scan_js_stage::ScanJsStage, scan_stage::ScanStage,
        },
    },
};

pub fn initialize_stages(allow_list: AllowList, output: PathBuf) -> (Dispatcher, JoinHandle<()>) {
    StageRegistry::default()
        .register(StageId::Filter, Box::new(FilterStage::new(allow_list)))
        .register(StageId::Map, Box::new(MapStage::new()))
        .register(StageId::SaveFile, Box::new(SaveFileStage::new(output)))
        .register(StageId::Scan, Box::new(ScanStage::new()))
        .register(StageId::JsScan, Box::new(ScanJsStage::new()))
        .build()
}
