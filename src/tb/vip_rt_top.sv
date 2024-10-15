module vip_rt_top #(
  parameter time             ClkPerSys   = 0ns,
  parameter time             ClkPerJtag  = 0ns,
  parameter longint unsigned RstClkCycles  = 0,
  parameter longint unsigned TimeoutCycles = 0
)(
  output logic clk_o,
  output logic rst_no,
  output logic jtag_tck_o,
  output logic jtag_tms_o,
  output logic jtag_trst_no,
  output logic jtag_tdi_o,
  input  logic jtag_tdo_i
);

import "DPI-C" function byte read_elf(input string filename);
import "DPI-C" function byte get_entry(output longint entry);
import "DPI-C" function byte get_section(output longint address, output longint len);
import "DPI-C" context function byte read_section(input longint address, inout byte buffer[], input longint len);

localparam real TAppl = 0.1;
localparam real TTest = 0.9;

logic clk, rst_n;

assign clk_o = clk;
assign rst_no = rst_n;

JTAG_DV jtag (jtag_tck_o);

localparam dm::sbcs_t JtagInitSbcs = dm::sbcs_t'{
  sbautoincrement: 1'b1, sbreadondata: 1'b1, sbaccess: 3, default: '0};

typedef jtag_test::riscv_dbg #(
  .IrLength ( 5 ),
  .TA       ( ClkPerJtag * TAppl ),
  .TT       ( ClkPerJtag * TTest )
) riscv_dbg_t;

typedef bit [31:0] word;

riscv_dbg_t::jtag_driver_t  jtag_dv   = new (jtag);
riscv_dbg_t                 jtag_dbg  = new (jtag_dv);

assign jtag_trst_no = jtag.trst_n;
assign jtag_tms_o   = jtag.tms;
assign jtag_tdi_o   = jtag.tdi;
assign jtag.tdo     = jtag_tdo_i;

initial begin
  @(negedge rst_n);
  jtag_dbg.reset_master();
end

clk_rst_gen #(
  .ClkPeriod    (ClkPerSys),
  .RstClkCycles (RstClkCycles)
) i_sys_clk_rst_gen (
  .clk_o  (clk),
  .rst_no (rst_n)
);

clk_rst_gen #(
  .ClkPeriod    (ClkPerJtag),
  .RstClkCycles (RstClkCycles)
) i_jtag_clk_rst_gen (
  .clk_o  (jtag_tck_o),
  .rst_no ()
);

//sim_timeout #(
//  .Cycles (TimeoutCycles),
//  .ResetRestartsTimeout (1)
//) i_watchdog (
//  .clk_i  (clk),
//  .rst_ni (rst_n)
//);

task automatic wait_for_reset;
  @(posedge rst_n);
  @(posedge clk);
endtask

task automatic jtag_write(
  input dm::dm_csr_e addr,
  input bit [31:0] data,
  input bit wait_cmd = 0,
  input bit wait_sba = 0
);
  jtag_dbg.write_dmi(addr, data);
  if (wait_cmd) begin
    dm::abstractcs_t acs;
    do begin
      jtag_dbg.read_dmi_exp_backoff(dm::AbstractCS, acs);
      if (acs.cmderr) $fatal(1, "[JTAG] Abstract command error!");
    end while (acs.busy);
  end
  if (wait_sba) begin
    dm::sbcs_t sbcs;
    do begin
      jtag_dbg.read_dmi_exp_backoff(dm::SBCS, sbcs);
      if (sbcs.sberror | sbcs.sbbusyerror) $fatal(1, "[JTAG] System bus error!");
    end while (sbcs.sbbusy);
  end
endtask

// Initialize the debug module
task automatic jtag_init;
  rt_pkg::jtag_idcode_t idcode;
  dm::dmcontrol_t dmcontrol = '{dmactive: 1, default: '0};
  // Check ID code
  repeat(100) @(posedge jtag_tck_o);
  jtag_dbg.get_idcode(idcode);
  if (idcode != rt_pkg::DbgIdCode)
      $fatal(1, "[JTAG] Unexpected ID code: expected 0x%h, got 0x%h!", rt_pkg::DbgIdCode, idcode);
  // Activate, wait for debug module
  jtag_write(dm::DMControl, dmcontrol);
  do jtag_dbg.read_dmi_exp_backoff(dm::DMControl, dmcontrol);
  while (~dmcontrol.dmactive);
  // Activate, wait for system bus
  jtag_write(dm::SBCS, JtagInitSbcs, 0, 1);
  $display("[JTAG] Initialization [PASSED]");
endtask

task automatic jtag_read_reg32(
  input  word addr,
  output word data,
  input int unsigned idle_cycles = 20,
  input bit verbose = 0
);
  automatic dm::sbcs_t sbcs = dm::sbcs_t'{sbreadonaddr: 1'b1, sbaccess: 2, default: '0};
  jtag_write(dm::SBCS, sbcs, 0, 1);
  jtag_write(dm::SBAddress1, addr[63:32]);
  jtag_write(dm::SBAddress0, addr[31:0]);
  jtag_dbg.wait_idle(idle_cycles);
  jtag_dbg.read_dmi_exp_backoff(dm::SBData0, data);
  if (verbose) $display("[JTAG] Read 0x%h from 0x%h", data, addr);
endtask

task automatic jtag_write_reg32(
  input word addr,
  input word data,
  input bit check_write,
  input int unsigned check_write_wait_cycles = 20,
  input bit verbose = 0
);
  automatic dm::sbcs_t sbcs = dm::sbcs_t'{sbaccess: 2, default: '0};
  if (verbose) $display("[JTAG] Writing 0x%h to 0x%h", data, addr);
  jtag_write(dm::SBCS, sbcs, 0, 1);
  jtag_write(dm::SBAddress1, addr[63:32]);
  jtag_write(dm::SBAddress0, addr[31:0]);
  jtag_write(dm::SBData0, data);
  jtag_dbg.wait_idle(check_write_wait_cycles);
  if (check_write) begin
    word rdata;
    jtag_read_reg32(addr, rdata, check_write_wait_cycles, verbose);
    if (rdata != data) $fatal(1,"[JTAG] - Read [FAILED], incorrect data 0x%h!", rdata);
    else if (verbose) $display("[JTAG] - Read back correct data");
  end
endtask

// Load a binary
task automatic jtag_elf_preload(input string binary, output word entry);
  longint sec_addr, sec_len;
  $display("[JTAG] Preloading ELF binary: %s", binary);
  if (read_elf(binary))
    $fatal(1, "[JTAG] Failed to load ELF!");
  while (get_section(sec_addr, sec_len)) begin
    byte bf[] = new [sec_len];
    $display("[JTAG] Preloading section at 0x%h (%0d bytes)", sec_addr, sec_len);
    if (read_section(sec_addr, bf, sec_len)) $fatal(1, "[JTAG] Failed to read ELF section!");
    jtag_write(dm::SBCS, JtagInitSbcs, 1, 1);
    // Write address as 64-bit double
    jtag_write(dm::SBAddress1, sec_addr[63:32]);
    jtag_write(dm::SBAddress0, sec_addr[31:0]);
    for (longint i = 0; i <= sec_len ; i += 8) begin
      bit checkpoint = (i != 0 && i % 512 == 0);
      if (checkpoint)
        $display("[JTAG] - %0d/%0d bytes (%0d%%)", i, sec_len, i*100/(sec_len>1 ? sec_len-1 : 1));
      jtag_write(dm::SBData1, {bf[i+7], bf[i+6], bf[i+5], bf[i+4]});
      jtag_write(dm::SBData0, {bf[i+3], bf[i+2], bf[i+1], bf[i]}, checkpoint, checkpoint);
    end
  end
  void'(get_entry(entry));
  $display("[JTAG] Preload complete");
endtask

// Halt the core and preload a binary
task automatic jtag_elf_halt_load(input string binary, output word entry);
  dm::dmstatus_t status;
  // Wait until bootrom initialized LLC
  //if (DutCfg.LlcNotBypass) begin
  //  word regval;
  //  $display("[JTAG] Wait for LLC configuration");
  //  jtag_poll_bit0(AmLlc + axi_llc_reg_pkg::AXI_LLC_CFG_SPM_LOW_OFFSET, regval, 20);
  //end
  // Halt hart 0
  jtag_write(dm::DMControl, dm::dmcontrol_t'{haltreq: 1, dmactive: 1, default: '0});
  do jtag_dbg.read_dmi_exp_backoff(dm::DMStatus, status);
  while (~status.allhalted);
  $display("[JTAG] Halted hart 0");
  // Preload binary
  jtag_elf_preload(binary, entry);
endtask

// Run a binary
task automatic jtag_elf_run(input string binary);
  word entry;
  jtag_elf_halt_load(binary, entry);
  // Repoint execution
  jtag_write(dm::Data1, entry[63:32]);
  jtag_write(dm::Data0, entry[31:0]);
  jtag_write(dm::Command, 32'h0033_07b1, 0, 1);
  // Resume hart 0
  jtag_write(dm::DMControl, dm::dmcontrol_t'{resumereq: 1, dmactive: 1, default: '0});
  $display("[JTAG] Resumed hart 0 from 0x%h", entry);
endtask

/*

// Wait for termination signal and get return code
task automatic jtag_wait_for_eoc(output word exit_code);
  jtag_poll_bit0(AmRegs + cheshire_reg_pkg::CHESHIRE_SCRATCH_2_OFFSET, exit_code, 800);
  exit_code >>= 1;
  if (exit_code) $error("[JTAG] FAILED: return code %0d", exit_code);
  else $display("[JTAG] SUCCESS");
endtask
*/


/// Sanity tests
task automatic run_dbg_mem_test();
  localparam int unsigned IterCnt = 200;
  int unsigned imem_start = rt_pkg::ObiXbarCfg.ImemStart;
  int unsigned dmem_end   = rt_pkg::ObiXbarCfg.DmemEnd;
  int unsigned size       = rt_pkg::get_addr_size(imem_start, dmem_end);
  int unsigned rand_addr  = 32'h0000_0000;

  $display("[test:dbg_mem] Memory size is %08H, range %08H to %08H", size, imem_start, dmem_end);
  $display("[test:dbg_mem] Performing %d word-alligned random address accesses", IterCnt);

  for (int i=0; i<IterCnt; i++) begin : alligned_loop
    rand_addr = (($urandom()%size) & 32'hFFFF_FFFC) + imem_start;
    jtag_write_reg32(rand_addr, $urandom(), 1, 20, 0);
  end : alligned_loop

  $display("[test:dbg_mem] No memory access errors, debugger test [PASSED]");

  //$display("[test:dbg_mem] Performing %d unalligned random address accesses", IterCnt);
  //for (int i=0; i<IterCnt; i++) begin : unalligned_loop
  //  rand_addr = ($urandom()%size) + imem_start;
  //  jtag_write_reg32(rand_addr, $urandom(), 1, 20, 0);
  //end : unalligned_loop
endtask

endmodule : vip_rt_top