use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Stroke};
use egui_plot::*;
use itertools::Itertools;
use sysinfo::*;

pub struct NetTracker {
    networks: Networks,
    last_refresh: Instant,
    refresh_interval: Duration,
    pub usage: f64,
    pub usage_history: Vec<f64>,
    pub should_track: bool,

}

impl NetTracker {

    pub fn new() -> Self {
        
        let networks = Networks::new_with_refreshed_list();

        //let (interface_name, net_data) = networks.into_iter().next().unwrap();

        let last_refresh = std::time::Instant::now();
        let refresh_interval = Duration::from_secs_f64(0.2);
        let usage = 0.0;
        let usage_history = vec![0.0, 0.0, 0.0, 0.0, 0.0];
        // for (interface_name, net_data) in &networks {
            
        // };

        NetTracker {
            networks,
            last_refresh,
            refresh_interval,
            usage,
            usage_history,
            should_track: false,

        }
        
    }

    pub fn start_tracking(&mut self) {

        self.should_track = true;
    }

    pub fn update(&mut self) {
        if !self.should_track {
            return;
        }

        if self.refresh_interval > self.last_refresh.elapsed() { return; }

        self.networks.refresh();
        for (_, net_data) in &self.networks  {
            self.usage = 8.0/1_000_000.0 * net_data.received() as f64 / self.last_refresh.elapsed().as_secs_f64();
        }
        self.usage_history.push(self.usage);
        self.last_refresh = std::time::Instant::now();
    
    }

    pub fn plot_usage_history(&self, ui: &mut egui::Ui) {


        
        //let smoothing = 5;
        let mut ys = Vec::new();
        for i in 4..self.usage_history.len() {
            ys.push(
                {
                    let us = self.usage_history.get(i-4..=i).unwrap();
                    us.into_iter().sum::<f64>() / 5.0
                }
            )
        }

        

        let line = Line::new(
            PlotPoints::from_ys_f64(&ys)
        )
        .color(Color32::LIGHT_GRAY)
        .style(LineStyle::Solid)
        .stroke(Stroke::new(2.0, Color32::LIGHT_GRAY));
        
        let plot = Plot::new("network_plot");

        let lp = plot.show(ui, |plot_ui|
            plot_ui.line(line)
        );

        
        
        
        
    }

}



