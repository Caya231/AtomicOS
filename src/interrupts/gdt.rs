use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor, SegmentSelector};
use x86_64::VirtAddr;
use lazy_static::lazy_static;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

/// Kernel stack used during syscalls/interrupts from Ring 3.
const KERNEL_STACK_SIZE: usize = 4096 * 5;
static mut KERNEL_STACK: [u8; KERNEL_STACK_SIZE] = [0; KERNEL_STACK_SIZE];

/// Double-fault handler stack.
const DF_STACK_SIZE: usize = 4096 * 5;
static mut DF_STACK: [u8; DF_STACK_SIZE] = [0; DF_STACK_SIZE];

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();

        // IST[0] = double-fault handler stack
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            let stack_start = VirtAddr::from_ptr(unsafe { &raw const DF_STACK });
            stack_start + DF_STACK_SIZE as u64
        };

        // RSP0 = kernel stack for Ring 3 → Ring 0 transitions
        tss.privilege_stack_table[0] = {
            let stack_start = VirtAddr::from_ptr(unsafe { &raw const KERNEL_STACK });
            stack_start + KERNEL_STACK_SIZE as u64
        };

        tss
    };
}

lazy_static! {
    pub static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let kernel_code = gdt.add_entry(Descriptor::kernel_code_segment());
        let kernel_data = gdt.add_entry(Descriptor::kernel_data_segment());
        let user_data   = gdt.add_entry(Descriptor::user_data_segment());
        let user_code   = gdt.add_entry(Descriptor::user_code_segment());
        let tss         = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (gdt, Selectors {
            kernel_code,
            kernel_data,
            user_code,
            user_data,
            tss,
        })
    };
}

pub struct Selectors {
    pub kernel_code: SegmentSelector,
    pub kernel_data: SegmentSelector,
    pub user_code: SegmentSelector,
    pub user_data: SegmentSelector,
    pub tss: SegmentSelector,
}

pub fn init() {
    use x86_64::instructions::tables::load_tss;
    use x86_64::instructions::segmentation::{CS, DS, SS, Segment};

    GDT.0.load();
    unsafe {
        CS::set_reg(GDT.1.kernel_code);
        DS::set_reg(GDT.1.kernel_data);
        SS::set_reg(GDT.1.kernel_data);
        load_tss(GDT.1.tss);
    }
}

/// Get the user code segment selector (with RPL=3).
pub fn user_code_selector() -> SegmentSelector {
    SegmentSelector::new(GDT.1.user_code.index(), x86_64::PrivilegeLevel::Ring3)
}

/// Get the user data segment selector (with RPL=3).
pub fn user_data_selector() -> SegmentSelector {
    SegmentSelector::new(GDT.1.user_data.index(), x86_64::PrivilegeLevel::Ring3)
}

/// Update the RSP0 field in the TSS so that the CPU uses the current task's
/// kernel stack when transitioning from Ring 3 to Ring 0.
pub fn set_tss_rsp0(kernel_stack_top: u64) {
    unsafe {
        // Cast away the const-ness of the lazy_static TSS reference
        // This is safe because we only call this with interrupts disabled
        // during a context switch, and the CPU reads this structure asynchronously.
        let tss_ptr = &*TSS as *const TaskStateSegment as *mut TaskStateSegment;
        (*tss_ptr).privilege_stack_table[0] = VirtAddr::new(kernel_stack_top);
    }
}
