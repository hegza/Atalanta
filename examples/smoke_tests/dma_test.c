#include <stdint.h>
#include <stdarg.h>
#include "include/csr_utils.h"
#include "include/uart_interrupt.h"

#define DMA_SRC 0x00006000
#define DMA_DST 0x00020000
#define DMA_LEN 0x20
#define DMA_CFG 0x00010000

// (pseudo)random data generation
uint32_t lfsr = 0xBEEFFACEu;
uint32_t bit;
uint32_t rand(){
  bit  = ((lfsr >> 0) ^ (lfsr >> 2) ^ (lfsr >> 3) ^ (lfsr >> 5) ) & 1;
  return lfsr =  (lfsr >> 1) | (bit << 31);
}

void init_buffer( uint32_t src, uint32_t len){
  for (int i = 0; i < DMA_LEN; i += 4){
    uint32_t tmp = rand();
    printf("[UART] Writing %x to addr %x\n", tmp, DMA_SRC + i);
    *(uint32_t*)(DMA_SRC + i) = tmp;
  }
}	

void dma_transfer( uint32_t src, uint32_t dst, uint32_t len){
  *(uint32_t*)(DMA_CFG    )  = 0x8;
  *(uint32_t*)(DMA_CFG + 4)  = DMA_DST;
  *(uint32_t*)(DMA_CFG + 4)  = DMA_SRC;
  *(uint32_t*)(DMA_CFG + 8)  = DMA_DST;
  *(uint32_t*)(DMA_CFG    ) |= 0x80000000;
}

int cmp_buffer( uint32_t src, uint32_t dst, uint32_t len){
  for (uint32_t i=0; i<len; i = i + 4){
    volatile uint32_t src_val = *(uint32_t*)(src + i);   
    volatile uint32_t dst_val = *(uint32_t*)(dst + i);   
    if (src_val != dst_val){
      printf("[UART] SRC-DST mismatch! SRC: 0x%x, DST 0x%x\n", src_val, dst_val);
      return 1; 
    }
  }
  printf("[UART] No mismatches, test [PASSED]\n");
}

int main() {  
  init_uart(100000000/2, 3000000); // 50 MHz for simulation, 30 MHz for FPGA
  printf("[UART] DMA test init: populate src buffer\n");
  init_buffer(DMA_SRC, DMA_LEN);

  printf("[UART] Call DMA transfer\n");
  dma_transfer(DMA_SRC, DMA_DST, DMA_LEN);

  printf("[UART] DMA test check: compare src and dst buffers\n");
  return cmp_buffer(DMA_SRC, DMA_DST, DMA_LEN);

}
