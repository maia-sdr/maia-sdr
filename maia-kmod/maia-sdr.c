/*
 * Copyright (C) 2022 Daniel Estevez <daniel@destevez.net>
 * 
 * This file forms part of maia-sdr
 *
 * SPDX-License-Identifier: GPL-2.0-only
 *
 */

#include <linux/cdev.h>
#include <linux/dma-mapping.h>
#include <linux/err.h>
#include <linux/fs.h>
#include <linux/init.h>
#include <linux/ioctl.h>
#include <linux/module.h>
#include <linux/of.h>
#include <linux/of_device.h>
#include <linux/of_reserved_mem.h>
#include <linux/platform_device.h>

#define DRIVER_NAME "maia-sdr"

static dev_t maia_sdr_device = 0;
static struct class *maia_sdr_class;
static int maia_sdr_platform_driver_registered = 0;

#define MAIA_SDR_MINOR_MAX 256
static DEFINE_IDA(maia_sdr_device_ida);

#define IOC_MAGIC 'M'
#define IOCTL_CACHEINV _IOW(IOC_MAGIC, 0, int)

struct maia_sdr_recording_drvdata {
	struct cdev cdev;
	dev_t device_number;
	struct device *device;
	phys_addr_t mem_base_addr;
	phys_addr_t mem_size;
};

struct maia_sdr_rxbuffer_drvdata {
	struct cdev cdev;
	dev_t device_number;
	struct device *device;
	phys_addr_t mem_base_addr;
	phys_addr_t buffer_size;
	unsigned int num_buffers;
	int mmap_done;
	unsigned long vm_start;
};

// Defined in arch/arm/mm/cache-v7.S
extern void v7_dma_inv_range(unsigned long start, unsigned long end);
// Defined in arch/arm/mm/outercache.c
extern void arm_cache_outer_inv_range(phys_addr_t start, phys_addr_t end);

static int maia_sdr_recording_open(struct inode *inode, struct file *file)
{
	struct maia_sdr_recording_drvdata *drvdata = container_of(
		inode->i_cdev, struct maia_sdr_recording_drvdata, cdev);
	file->private_data = drvdata;
	return 0;
}

static int maia_sdr_recording_mmap(struct file *file,
				   struct vm_area_struct *vma)
{
	const struct maia_sdr_recording_drvdata *drvdata = file->private_data;
	unsigned long size = vma->vm_end - vma->vm_start;
	phys_addr_t offset = (phys_addr_t)vma->vm_pgoff << PAGE_SHIFT;
	int ret;
	const unsigned long pte_mask = L_PTE_RDONLY | L_PTE_XN;

	if ((pgprot_val(vma->vm_page_prot) & pte_mask) != pte_mask) {
		// The pages can only be mapped in read-only no-execute mode
		return -EPERM;
	}

	if ((offset > drvdata->mem_size) ||
	    (size > drvdata->mem_size - offset)) {
		// requested mapping is too large
		return -EINVAL;
	}

	ret = remap_pfn_range(vma, vma->vm_start,
			      (drvdata->mem_base_addr >> PAGE_SHIFT) +
				      vma->vm_pgoff,
			      size, vma->vm_page_prot);
	if (ret) {
		return ret;
	}

	// Invalidate ARMv7 L1 cache by virtual address range.
	//
	// The L1 data cache is physically indexed and physically tagged, but
	// invalidation is done using virtual addresses, line by line. We use
	// the userspace virtual addresses, since the DMA buffer is not mapped
	// at all in kernel memory.
	v7_dma_inv_range(vma->vm_start, vma->vm_end);

	// Invalidate L2 cache by physical address range.
	arm_cache_outer_inv_range(drvdata->mem_base_addr + offset,
				  drvdata->mem_base_addr + offset + size);

	return 0;
}

static void maia_sdr_rxbuffer_vm_close(struct vm_area_struct *vma)
{
	struct maia_sdr_rxbuffer_drvdata *drvdata = vma->vm_private_data;
	drvdata->mmap_done = 0;
}

static struct vm_operations_struct maia_sdr_rxbuffer_vm_ops = {
	.close = maia_sdr_rxbuffer_vm_close,
};

static int maia_sdr_rxbuffer_mmap(struct file *file, struct vm_area_struct *vma)
{
	struct maia_sdr_rxbuffer_drvdata *drvdata = file->private_data;
	unsigned long size = vma->vm_end - vma->vm_start;
	phys_addr_t offset = (phys_addr_t)vma->vm_pgoff << PAGE_SHIFT;
	int ret;
	const unsigned long pte_mask = L_PTE_RDONLY | L_PTE_XN;
	unsigned long max_size = drvdata->buffer_size * drvdata->num_buffers;

	if (drvdata->mmap_done) {
		// Device is already mapped
		return -EINVAL;
	}

	if ((pgprot_val(vma->vm_page_prot) & pte_mask) != pte_mask) {
		// The pages can only be mapped in read-only no-execute mode
		return -EPERM;
	}

	if ((offset > max_size) || (size > max_size - offset)) {
		// requested mapping is too large
		return -EINVAL;
	}

	ret = remap_pfn_range(vma, vma->vm_start,
			      (drvdata->mem_base_addr >> PAGE_SHIFT) +
				      vma->vm_pgoff,
			      size, vma->vm_page_prot);
	if (ret) {
		return ret;
	}
	drvdata->mmap_done = 1;
	drvdata->vm_start = vma->vm_start;
	vma->vm_private_data = (void *)drvdata;
	vma->vm_ops = &maia_sdr_rxbuffer_vm_ops;

	return 0;
}

static int
maia_sdr_rxbuffer_cacheinv(const struct maia_sdr_rxbuffer_drvdata *drvdata,
			   unsigned num_buffer)
{
	unsigned long offset;
	unsigned long start;
	if (num_buffer >= drvdata->num_buffers) {
		return -1;
	}
	offset = drvdata->buffer_size * num_buffer;
	start = drvdata->vm_start + offset;
	// See maia_sdr_recording_mmap for how the cache invalidation works.
	v7_dma_inv_range(start, start + drvdata->buffer_size);
	start = drvdata->mem_base_addr + offset;
	arm_cache_outer_inv_range(start, start + drvdata->buffer_size);
	return 0;
}

static long maia_sdr_rxbuffer_ioctl(struct file *file, unsigned int cmd,
				    unsigned long arg)
{
	struct maia_sdr_rxbuffer_drvdata *drvdata = file->private_data;

	switch (cmd) {
	case IOCTL_CACHEINV:
		return maia_sdr_rxbuffer_cacheinv(drvdata, arg);
	default:
		return -ENOTTY;
	}
	return 0;
}

static int maia_sdr_rxbuffer_open(struct inode *inode, struct file *file)
{
	struct maia_sdr_rxbuffer_drvdata *drvdata = container_of(
		inode->i_cdev, struct maia_sdr_rxbuffer_drvdata, cdev);
	file->private_data = drvdata;
	return 0;
}

static ssize_t recording_base_address_show(struct device *dev,
					   struct device_attribute *attr,
					   char *buf)
{
	struct maia_sdr_recording_drvdata *drvdata = dev_get_drvdata(dev);
	return sprintf(buf, "0x%08x\n", drvdata->mem_base_addr);
}

static ssize_t recording_size_show(struct device *dev,
				   struct device_attribute *attr, char *buf)
{
	struct maia_sdr_recording_drvdata *drvdata = dev_get_drvdata(dev);
	return sprintf(buf, "0x%08x\n", drvdata->mem_size);
}

static ssize_t rxbuffer_size_show(struct device *dev,
				  struct device_attribute *attr, char *buf)
{
	struct maia_sdr_rxbuffer_drvdata *drvdata = dev_get_drvdata(dev);
	return sprintf(buf, "0x%08x\n", drvdata->buffer_size);
}

static ssize_t rxbuffer_num_buffers_show(struct device *dev,
					 struct device_attribute *attr,
					 char *buf)
{
	struct maia_sdr_rxbuffer_drvdata *drvdata = dev_get_drvdata(dev);
	return sprintf(buf, "%d\n", drvdata->num_buffers);
}

static DEVICE_ATTR(recording_base_address, 0444, recording_base_address_show,
		   NULL);
static DEVICE_ATTR(recording_size, 0444, recording_size_show, NULL);
static DEVICE_ATTR(buffer_size, 0444, rxbuffer_size_show, NULL);
static DEVICE_ATTR(num_buffers, 0444, rxbuffer_num_buffers_show, NULL);

static struct file_operations recording_fops = {
	.owner = THIS_MODULE,
	.open = maia_sdr_recording_open,
	.mmap = maia_sdr_recording_mmap,
};

static struct file_operations rxbuffer_fops = {
	.owner = THIS_MODULE,
	.open = maia_sdr_rxbuffer_open,
	.mmap = maia_sdr_rxbuffer_mmap,
	.unlocked_ioctl = maia_sdr_rxbuffer_ioctl,
};

enum maia_sdr_device_type {
	MAIA_SDR_RECORDING,
	MAIA_SDR_RXBUFFER,
};

struct maia_sdr_device_data {
	enum maia_sdr_device_type type;
};

static struct maia_sdr_device_data maia_sdr_devdata[] = {
        [MAIA_SDR_RECORDING] = {
                .type = MAIA_SDR_RECORDING,
        },
        [MAIA_SDR_RXBUFFER] = {
          .type = MAIA_SDR_RXBUFFER,
        },
};

static int maia_sdr_probe_recording(struct platform_device *pdev)
{
	int ret = 0;
	struct maia_sdr_recording_drvdata *drvdata;
	struct device_node *memory_region;
	struct reserved_mem *reserved_mem;
	int minor = -1;
	int cdev_add_done = 0;
	int create_base_addr_done = 0;
	int create_size_done = 0;

	drvdata = devm_kzalloc(&pdev->dev, sizeof(*drvdata), GFP_KERNEL);
	if (IS_ERR_OR_NULL(drvdata)) {
		return -ENOMEM;
	}

	memory_region = of_parse_phandle(pdev->dev.of_node, "memory-region", 0);
	if (!memory_region) {
		ret = -ENODEV;
		pr_alert("memory-region not found\n");
		goto probe_recording_error;
	}
	reserved_mem = of_reserved_mem_lookup(memory_region);
	if (!reserved_mem) {
		ret = -ENODEV;
		pr_alert("of_reserved_mem_lookup failed\n");
		goto probe_recording_error;
	}
	drvdata->mem_base_addr = reserved_mem->base;
	drvdata->mem_size = reserved_mem->size;

	cdev_init(&drvdata->cdev, &recording_fops);
	drvdata->cdev.owner = THIS_MODULE;
	minor = ida_simple_get(&maia_sdr_device_ida, 0, MAIA_SDR_MINOR_MAX,
			       GFP_KERNEL);
	if (minor < 0) {
		ret = minor;
		goto probe_recording_error;
	}
	drvdata->device_number = MKDEV(MAJOR(maia_sdr_device), minor);
	ret = cdev_add(&drvdata->cdev, drvdata->device_number, 1);
	if (ret < 0) {
		pr_alert("cdev_add failed with %d\n", ret);
		goto probe_recording_error;
	}
	cdev_add_done = 1;

	drvdata->device = device_create(maia_sdr_class, &pdev->dev,
					drvdata->device_number, drvdata,
					pdev->dev.of_node->name);
	if (IS_ERR_OR_NULL(drvdata->device)) {
		ret = drvdata->device ? PTR_ERR(drvdata->device) : -ENOMEM;
		pr_alert("device_create failed with %d\n", ret);
		goto probe_recording_error;
	}

	ret = device_create_file(&pdev->dev, &dev_attr_recording_base_address);
	if (ret < 0) {
		pr_alert("device_create_file failed with %d\n", ret);
		goto probe_recording_error;
	}
	create_base_addr_done = 1;

	ret = device_create_file(&pdev->dev, &dev_attr_recording_size);
	if (ret < 0) {
		pr_alert("device_create_file failed with %d\n", ret);
		goto probe_recording_error;
	}
	create_size_done = 1;

	platform_set_drvdata(pdev, drvdata);

	return 0;
probe_recording_error:
	if (create_base_addr_done) {
		device_remove_file(&pdev->dev,
				   &dev_attr_recording_base_address);
	}
	if (create_size_done) {
		device_remove_file(&pdev->dev, &dev_attr_recording_size);
	}
	if (!IS_ERR_OR_NULL(drvdata) && !IS_ERR_OR_NULL(drvdata->device)) {
		device_destroy(maia_sdr_class, drvdata->device_number);
	}
	if (cdev_add_done) {
		cdev_del(&drvdata->cdev);
	}
	if (minor >= 0) {
		ida_simple_remove(&maia_sdr_device_ida, minor);
	}
	if (!IS_ERR_OR_NULL(drvdata)) {
		devm_kfree(&pdev->dev, drvdata);
	}
	return ret;
}

static int maia_sdr_probe_rxbuffer(struct platform_device *pdev)
{
	int ret = 0;
	struct maia_sdr_rxbuffer_drvdata *drvdata;
	struct device_node *memory_region;
	struct reserved_mem *reserved_mem;
	u32 buffer_size;
	int minor = -1;
	int cdev_add_done = 0;
	int create_buffer_size_done = 0;
	int create_num_buffers_done = 0;

	drvdata = devm_kzalloc(&pdev->dev, sizeof(*drvdata), GFP_KERNEL);
	if (IS_ERR_OR_NULL(drvdata)) {
		return -ENOMEM;
	}

	ret = of_property_read_u32(pdev->dev.of_node, "buffer-size",
				   &buffer_size);
	if (ret < 0) {
		dev_err(&pdev->dev, "buffer-size property missing\n");
		goto probe_rxbuffer_error;
	}
	drvdata->buffer_size = (phys_addr_t)buffer_size;

	memory_region = of_parse_phandle(pdev->dev.of_node, "memory-region", 0);
	if (!memory_region) {
		ret = -ENODEV;
		pr_alert("memory-region not found\n");
		goto probe_rxbuffer_error;
	}
	reserved_mem = of_reserved_mem_lookup(memory_region);
	if (!reserved_mem) {
		ret = -ENODEV;
		pr_alert("of_reserved_mem_lookup failed\n");
		goto probe_rxbuffer_error;
	}
	if (reserved_mem->size % drvdata->buffer_size != 0) {
		ret = -EINVAL;
		dev_err(&pdev->dev,
			"memory-region size is not divisible by buffer-size\n");
		goto probe_rxbuffer_error;
	}
	drvdata->mem_base_addr = reserved_mem->base;
	drvdata->num_buffers = reserved_mem->size / drvdata->buffer_size;

	cdev_init(&drvdata->cdev, &rxbuffer_fops);
	drvdata->cdev.owner = THIS_MODULE;
	minor = ida_simple_get(&maia_sdr_device_ida, 0, MAIA_SDR_MINOR_MAX,
			       GFP_KERNEL);
	if (minor < 0) {
		ret = minor;
		goto probe_rxbuffer_error;
	}
	drvdata->device_number = MKDEV(MAJOR(maia_sdr_device), minor);
	ret = cdev_add(&drvdata->cdev, drvdata->device_number, 1);
	if (ret < 0) {
		pr_alert("cdev_add failed with %d\n", ret);
		goto probe_rxbuffer_error;
	}
	cdev_add_done = 1;

	drvdata->device = device_create(maia_sdr_class, &pdev->dev,
					drvdata->device_number, drvdata,
					pdev->dev.of_node->name);
	if (IS_ERR_OR_NULL(drvdata->device)) {
		ret = drvdata->device ? PTR_ERR(drvdata->device) : -ENOMEM;
		pr_alert("device_create failed with %d\n", ret);
		goto probe_rxbuffer_error;
	}

	ret = device_create_file(&pdev->dev, &dev_attr_buffer_size);
	if (ret < 0) {
		pr_alert("device_create_file failed with %d\n", ret);
		goto probe_rxbuffer_error;
	}
	create_buffer_size_done = 1;

	ret = device_create_file(&pdev->dev, &dev_attr_num_buffers);
	if (ret < 0) {
		pr_alert("device_create_file failed with %d\n", ret);
		goto probe_rxbuffer_error;
	}
	create_num_buffers_done = 1;

	platform_set_drvdata(pdev, drvdata);

	return 0;

probe_rxbuffer_error:
	if (create_num_buffers_done) {
		device_remove_file(&pdev->dev, &dev_attr_num_buffers);
	}
	if (create_buffer_size_done) {
		device_remove_file(&pdev->dev, &dev_attr_buffer_size);
	}
	if (!IS_ERR_OR_NULL(drvdata) && !IS_ERR_OR_NULL(drvdata->device)) {
		device_destroy(maia_sdr_class, drvdata->device_number);
	}
	if (cdev_add_done) {
		cdev_del(&drvdata->cdev);
	}
	if (minor >= 0) {
		ida_simple_remove(&maia_sdr_device_ida, minor);
	}
	if (!IS_ERR_OR_NULL(drvdata)) {
		devm_kfree(&pdev->dev, drvdata);
	}
	return ret;
}

static const struct of_device_id maia_sdr_of_match[] = {
	{
		.compatible = "maia-sdr,recording",
		.data = &maia_sdr_devdata[MAIA_SDR_RECORDING],
	},
	{
		.compatible = "maia-sdr,rxbuffer",
		.data = &maia_sdr_devdata[MAIA_SDR_RXBUFFER],
	},
	{ /* end of table */ },
};

static int maia_sdr_probe(struct platform_device *pdev)
{
	const struct maia_sdr_device_data *devdata;
	const struct of_device_id *of_id =
		of_match_device(of_match_ptr(maia_sdr_of_match), &pdev->dev);
	if (!of_id) {
		pr_alert("no of_match_device found");
		return -EINVAL;
	}
	devdata = of_id->data;
	switch (devdata->type) {
	case MAIA_SDR_RECORDING:
		return maia_sdr_probe_recording(pdev);
	case MAIA_SDR_RXBUFFER:
		return maia_sdr_probe_rxbuffer(pdev);
	default:
		pr_alert("unsupported device type");
		return -EINVAL;
	}
}

static int maia_sdr_remove_recording(struct platform_device *pdev)
{
	struct maia_sdr_recording_drvdata *drvdata = platform_get_drvdata(pdev);

	device_remove_file(&pdev->dev, &dev_attr_recording_base_address);
	device_remove_file(&pdev->dev, &dev_attr_recording_size);
	device_destroy(maia_sdr_class, drvdata->device_number);
	cdev_del(&drvdata->cdev);
	ida_simple_remove(&maia_sdr_device_ida, MINOR(drvdata->device_number));
	devm_kfree(&pdev->dev, drvdata);
	return 0;
}

static int maia_sdr_remove_rxbuffer(struct platform_device *pdev)
{
	struct maia_sdr_rxbuffer_drvdata *drvdata = platform_get_drvdata(pdev);

	device_remove_file(&pdev->dev, &dev_attr_num_buffers);
	device_remove_file(&pdev->dev, &dev_attr_buffer_size);
	device_destroy(maia_sdr_class, drvdata->device_number);
	cdev_del(&drvdata->cdev);
	ida_simple_remove(&maia_sdr_device_ida, MINOR(drvdata->device_number));
	devm_kfree(&pdev->dev, drvdata);
	return 0;
}

static int maia_sdr_remove(struct platform_device *pdev)
{
	const struct maia_sdr_device_data *devdata;
	const struct of_device_id *of_id =
		of_match_device(of_match_ptr(maia_sdr_of_match), &pdev->dev);
	if (!of_id) {
		pr_alert("no of_match_device found");
		return -EINVAL;
	}
	devdata = of_id->data;
	switch (devdata->type) {
	case MAIA_SDR_RECORDING:
		return maia_sdr_remove_recording(pdev);
	case MAIA_SDR_RXBUFFER:
		return maia_sdr_remove_rxbuffer(pdev);
	default:
		pr_alert("unsupported device type");
		return -EINVAL;
	}
}

MODULE_DEVICE_TABLE(of, maia_sdr_of_match);

static struct platform_driver maia_sdr_platform_driver = {
        .probe = maia_sdr_probe,
        .remove = maia_sdr_remove,
        .driver = {
                .owner = THIS_MODULE,
                .name = DRIVER_NAME,
                .of_match_table = of_match_ptr(maia_sdr_of_match),
        },
};

static void maia_sdr_cleanup(void)
{
	if (maia_sdr_platform_driver_registered) {
		platform_driver_unregister(&maia_sdr_platform_driver);
	}
	if (!IS_ERR_OR_NULL(maia_sdr_class)) {
		class_destroy(maia_sdr_class);
	}
	if (maia_sdr_device) {
		unregister_chrdev_region(maia_sdr_device, 0);
	}
}

static int __init maia_sdr_init(void)
{
	int ret = 0;

	ret = alloc_chrdev_region(&maia_sdr_device, 0, 0, DRIVER_NAME);
	if (ret < 0) {
		pr_alert("%s: alloc_chrdev_region failed with %d\n",
			 DRIVER_NAME, ret);
		maia_sdr_device = 0;
		goto error;
	}

	maia_sdr_class = class_create(THIS_MODULE, DRIVER_NAME);
	if (IS_ERR_OR_NULL(maia_sdr_class)) {
		ret = maia_sdr_class ? PTR_ERR(maia_sdr_class) : -ENOMEM;
		pr_alert("%s: class_create failed with %d\n", DRIVER_NAME, ret);
		goto error;
	}

	ret = platform_driver_register(&maia_sdr_platform_driver);
	if (ret < 0) {
		pr_alert("%s: platform_driver_register failed with %d\n",
			 DRIVER_NAME, ret);
		goto error;
	}
	maia_sdr_platform_driver_registered = 1;

	return 0;
error:
	maia_sdr_cleanup();
	return ret;
}

static void __exit maia_sdr_exit(void)
{
	maia_sdr_cleanup();
}

module_init(maia_sdr_init);
module_exit(maia_sdr_exit);

MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("Maia SDR kernel module");
MODULE_AUTHOR("Daniel Estevez <daniel@destevez.net>");
