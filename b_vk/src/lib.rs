use self::utils::{
    create_surface, enumerate_required_extensions, record_submit_commandbuffer,
    vulkan_debug_callback,
};
use ash::{
    ext::debug_utils,
    khr::{surface, swapchain},
    vk::{self, API_VERSION_1_3},
    Device, Entry, Instance,
};
use jester_core::{Backend, SpriteBatch, SpriteInstance, TextureId, MAX_SPRITES, MAX_TEXTURES};
use std::{ffi, os::raw::c_char};
use tracing::info;
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

#[repr(C)]
#[derive(Clone, Copy)]
struct QuadVertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

const QUAD_VERTS: [QuadVertex; 4] = [
    QuadVertex {
        pos: [-0.5, -0.5],
        uv: [0.0, 0.0],
    },
    QuadVertex {
        pos: [0.5, -0.5],
        uv: [1.0, 0.0],
    },
    QuadVertex {
        pos: [-0.5, 0.5],
        uv: [0.0, 1.0],
    },
    QuadVertex {
        pos: [0.5, 0.5],
        uv: [1.0, 1.0],
    },
];

mod utils;

pub struct VkBackend {
    pub entry: Entry,
    pub instance: Instance,
    pub device: Device,
    pub surface_loader: surface::Instance,
    pub swapchain_loader: swapchain::Device,
    #[cfg(feature = "debug")]
    pub debug_utils_loader: debug_utils::Instance,
    #[cfg(feature = "debug")]
    pub debug_call_back: vk::DebugUtilsMessengerEXT,

    pub pdevice: vk::PhysicalDevice,
    pub device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub queue_family_index: u32,
    pub present_queue: vk::Queue,

    pub surface: vk::SurfaceKHR,
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_resolution: vk::Extent2D,

    pub swapchain: vk::SwapchainKHR,
    pub present_images: Vec<vk::Image>,
    pub present_image_views: Vec<vk::ImageView>,

    pub pool: vk::CommandPool,
    pub cmds: Vec<vk::CommandBuffer>,

    pub render_pass: vk::RenderPass,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub current_img: usize,
    pub image_available: [vk::Semaphore; Self::MAX_FRAMES_IN_FLIGHT],
    pub render_finished: Vec<vk::Semaphore>,
    pub in_flight_fence: [vk::Fence; Self::MAX_FRAMES_IN_FLIGHT],

    pub frame_idx: usize,

    // misc
    pub swapchain_rebuild: bool,

    // pipeline
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,

    pub quad_vbo: vk::Buffer,
    pub quad_vbo_mem: vk::DeviceMemory,

    pub instance_vbo: vk::Buffer,
    pub instance_vbo_mem: vk::DeviceMemory,

    // textures
    pub images: Vec<vk::Image>,
    pub image_mem: Vec<vk::DeviceMemory>,
    pub image_views: Vec<vk::ImageView>,
    pub samplers: Vec<vk::Sampler>,
    pub descriptor_sets: Vec<vk::DescriptorSet>, // ← one per texture

    // common objects
    pub desc_set_layout: vk::DescriptorSetLayout,
    pub desc_pool: vk::DescriptorPool,

    pub instance_cursor: vk::DeviceSize,
}

impl VkBackend {
    const MAX_FRAMES_IN_FLIGHT: usize = 2;

    fn create_swapchain(
        &mut self,
        window_width: u32,
        window_height: u32,
    ) -> Result<(), vk::Result> {
        unsafe {
            let caps = self
                .surface_loader
                .get_physical_device_surface_capabilities(self.pdevice, self.surface)?;

            let formats = self
                .surface_loader
                .get_physical_device_surface_formats(self.pdevice, self.surface)?;
            self.surface_format = formats[0];

            let present_modes = self
                .surface_loader
                .get_physical_device_surface_present_modes(self.pdevice, self.surface)?;
            let present_mode = present_modes
                .iter()
                .cloned()
                .find(|m| *m == vk::PresentModeKHR::MAILBOX)
                .unwrap_or(vk::PresentModeKHR::FIFO);

            let desired_image_count =
                (caps.min_image_count + 1).min(caps.max_image_count.max(caps.min_image_count + 1));

            self.surface_resolution = match caps.current_extent.width {
                u32::MAX => vk::Extent2D {
                    width: window_width,
                    height: window_height,
                },
                _ => caps.current_extent,
            };

            for &fb in &self.framebuffers {
                self.device.destroy_framebuffer(fb, None);
            }
            for &view in &self.present_image_views {
                self.device.destroy_image_view(view, None);
            }
            for &sem in &self.render_finished {
                self.device.destroy_semaphore(sem, None);
            }
            if self.swapchain != vk::SwapchainKHR::null() {
                self.swapchain_loader
                    .destroy_swapchain(self.swapchain, None);
            }

            let swap_info = vk::SwapchainCreateInfoKHR::default()
                .surface(self.surface)
                .min_image_count(desired_image_count)
                .image_color_space(self.surface_format.color_space)
                .image_format(self.surface_format.format)
                .image_extent(self.surface_resolution)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(
                    if caps
                        .supported_transforms
                        .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
                    {
                        vk::SurfaceTransformFlagsKHR::IDENTITY
                    } else {
                        caps.current_transform
                    },
                )
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(present_mode)
                .clipped(true)
                .image_array_layers(1);

            self.swapchain = self.swapchain_loader.create_swapchain(&swap_info, None)?;

            self.present_images = self.swapchain_loader.get_swapchain_images(self.swapchain)?;
            self.present_image_views = self
                .present_images
                .iter()
                .map(|&img| {
                    let view_info = vk::ImageViewCreateInfo::default()
                        .image(img)
                        .view_type(vk::ImageViewType::TYPE_2D)
                        .format(self.surface_format.format)
                        .subresource_range(
                            vk::ImageSubresourceRange::default()
                                .aspect_mask(vk::ImageAspectFlags::COLOR)
                                .layer_count(1)
                                .level_count(1),
                        );
                    self.device.create_image_view(&view_info, None)
                })
                .collect::<Result<_, _>>()?;

            let sem_info = vk::SemaphoreCreateInfo::default();
            self.render_finished = self
                .present_images
                .iter()
                .map(|_| self.device.create_semaphore(&sem_info, None))
                .collect::<Result<_, _>>()?;

            self.framebuffers = self
                .present_image_views
                .iter()
                .map(|&view| {
                    let fb_info = vk::FramebufferCreateInfo::default()
                        .render_pass(self.render_pass)
                        .attachments(std::slice::from_ref(&view))
                        .width(self.surface_resolution.width)
                        .height(self.surface_resolution.height)
                        .layers(1);
                    self.device.create_framebuffer(&fb_info, None)
                })
                .collect::<Result<_, _>>()?;

            Ok(())
        }
    }
}

impl Backend for VkBackend {
    type Error = vk::Result;

    fn handle_resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width == self.surface_resolution.width
            && size.height == self.surface_resolution.height
        {
            return;
        }
        self.swapchain_rebuild = true;
    }

    fn create_texture(
        &mut self,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Result<TextureId, vk::Result> {
        assert_eq!(
            pixels.len(),
            (width * height * 4) as usize,
            "pixels buffer must be RGBA-8 per texel"
        );
        if self.images.len() >= MAX_TEXTURES {
            panic!("texture limit reached ({MAX_TEXTURES})");
        }

        let img_size = pixels.len() as vk::DeviceSize;
        let (stage_buf, stage_mem) = shaders::create_buffer(
            &self.device,
            &self.device_memory_properties,
            img_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        unsafe {
            let dst = self
                .device
                .map_memory(stage_mem, 0, img_size, vk::MemoryMapFlags::empty())?
                as *mut u8;
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), dst, pixels.len());
            self.device.unmap_memory(stage_mem);
        }

        let img_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let image = unsafe { self.device.create_image(&img_info, None)? };

        let req = unsafe { self.device.get_image_memory_requirements(image) };
        let mem_index = utils::find_memorytype_index(
            &req,
            &self.device_memory_properties,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .expect("no device-local memory for texture");

        let alloc = vk::MemoryAllocateInfo::default()
            .allocation_size(req.size)
            .memory_type_index(mem_index);

        let image_mem = unsafe { self.device.allocate_memory(&alloc, None)? };
        unsafe { self.device.bind_image_memory(image, image_mem, 0)? };

        let tmp_cmd = unsafe {
            self.device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(self.pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1),
            )?[0]
        };
        let tmp_fence = unsafe {
            self.device.create_fence(
                &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )?
        };

        let region = vk::BufferImageCopy::default()
            .buffer_offset(0)
            .image_subresource(
                vk::ImageSubresourceLayers::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .layer_count(1),
            )
            .image_extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            });

        record_submit_commandbuffer(
            &self.device,
            tmp_cmd,
            tmp_fence,
            self.present_queue,
            &[],
            &[],
            &[],
            |d, c| unsafe {
                let to_transfer = vk::ImageMemoryBarrier::default()
                    .image(image)
                    .src_access_mask(vk::AccessFlags::empty())
                    .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .level_count(1)
                            .layer_count(1),
                    );
                d.cmd_pipeline_barrier(
                    c,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[to_transfer],
                );

                d.cmd_copy_buffer_to_image(
                    c,
                    stage_buf,
                    image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    std::slice::from_ref(&region),
                );

                let to_shader = vk::ImageMemoryBarrier::default()
                    .image(image)
                    .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .dst_access_mask(vk::AccessFlags::SHADER_READ)
                    .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .level_count(1)
                            .layer_count(1),
                    );
                d.cmd_pipeline_barrier(
                    c,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[to_shader],
                );
            },
        );

        unsafe {
            self.device.wait_for_fences(&[tmp_fence], true, u64::MAX)?;
            self.device.destroy_fence(tmp_fence, None);
            self.device.free_command_buffers(self.pool, &[tmp_cmd]);
            self.device.destroy_buffer(stage_buf, None);
            self.device.free_memory(stage_mem, None);
        }

        let view = unsafe {
            self.device.create_image_view(
                &vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(vk::Format::R8G8B8A8_UNORM)
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .level_count(1)
                            .layer_count(1),
                    ),
                None,
            )?
        };

        // LINEAR SAMPLING
        // let sampler = unsafe {
        //     self.device.create_sampler(
        //         &vk::SamplerCreateInfo::default()
        //             .min_filter(vk::Filter::LINEAR)
        //             .mag_filter(vk::Filter::LINEAR)
        //             .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
        //             .address_mode_u(vk::SamplerAddressMode::REPEAT)
        //             .address_mode_v(vk::SamplerAddressMode::REPEAT)
        //             .max_lod(0.0),
        //         None,
        //     )?
        // };

        // NEAREST SAMPLING
        let sampler = unsafe {
            self.device.create_sampler(
                &vk::SamplerCreateInfo::default()
                    .min_filter(vk::Filter::NEAREST)
                    .mag_filter(vk::Filter::NEAREST)
                    .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
                    .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .max_lod(0.0),
                None,
            )?
        };

        let desc_set = unsafe {
            self.device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::default()
                    .descriptor_pool(self.desc_pool)
                    .set_layouts(std::slice::from_ref(&self.desc_set_layout)),
            )?[0]
        };

        let img_info = vk::DescriptorImageInfo::default()
            .sampler(sampler)
            .image_view(view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

        let write = vk::WriteDescriptorSet::default()
            .dst_set(desc_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(std::slice::from_ref(&img_info));

        unsafe {
            self.device
                .update_descriptor_sets(std::slice::from_ref(&write), &[]);
        }

        /* ---------- keep handles ------------------------------------------- */
        self.images.push(image);
        self.image_mem.push(image_mem);
        self.image_views.push(view);
        self.samplers.push(sampler);
        self.descriptor_sets.push(desc_set);

        Ok(TextureId((self.descriptor_sets.len() - 1) as u32))
    }

    fn begin_frame(&mut self) {
        if self.swapchain_rebuild {
            unsafe { self.device.device_wait_idle() }.unwrap();
            let _ = self.create_swapchain(
                self.surface_resolution.width,
                self.surface_resolution.height,
            );
            self.swapchain_rebuild = false;
        }
        let fi = self.frame_idx;
        let cmd = self.cmds[fi];
        unsafe {
            self.device
                .wait_for_fences(&[self.in_flight_fence[fi]], true, u64::MAX)
                .expect("Wait for fence failed.");
            self.device
                .reset_fences(&[self.in_flight_fence[fi]])
                .expect("Reset fences failed.");
        }

        let (img_index, _) = unsafe {
            self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                self.image_available[fi],
                vk::Fence::null(),
            )
        }
        .unwrap();
        self.current_img = img_index as usize;

        unsafe {
            self.device
                .reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())
                .unwrap();

            let begin_info = vk::CommandBufferBeginInfo::default();
            self.device.begin_command_buffer(cmd, &begin_info).unwrap();

            let vp = vk::Viewport::default()
                .width(self.surface_resolution.width as f32)
                .height(self.surface_resolution.height as f32)
                .min_depth(0.0)
                .max_depth(1.0);
            let sc = vk::Rect2D::default().extent(self.surface_resolution);
            self.device
                .cmd_set_viewport(cmd, 0, std::slice::from_ref(&vp));
            self.device
                .cmd_set_scissor(cmd, 0, std::slice::from_ref(&sc));

            let clear = vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.05, 0.05, 0.09, 1.0],
                },
            };
            self.device.cmd_begin_render_pass(
                cmd,
                &vk::RenderPassBeginInfo::default()
                    .render_pass(self.render_pass)
                    .framebuffer(self.framebuffers[self.current_img])
                    .render_area(vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: self.surface_resolution,
                    })
                    .clear_values(std::slice::from_ref(&clear)),
                vk::SubpassContents::INLINE,
            );
            let screen = [
                self.surface_resolution.width as f32,
                self.surface_resolution.height as f32,
            ];
            self.device.cmd_push_constants(
                self.cmds[self.frame_idx],
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                bytemuck::bytes_of(&screen),
            );
        }
        self.instance_cursor = 0;
    }

    fn end_frame(&mut self) {
        let fi = self.frame_idx;
        let img = self.current_img;
        let cmd = self.cmds[fi];
        let rf_sema = self.render_finished[img];

        unsafe {
            self.device.cmd_end_render_pass(cmd);
            self.device.end_command_buffer(cmd).unwrap();

            let submit = vk::SubmitInfo::default()
                .wait_semaphores(std::slice::from_ref(&self.image_available[fi]))
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                .command_buffers(std::slice::from_ref(&cmd))
                .signal_semaphores(std::slice::from_ref(&rf_sema));

            self.device
                .queue_submit(
                    self.present_queue,
                    std::slice::from_ref(&submit),
                    self.in_flight_fence[fi],
                )
                .unwrap();

            let img_u32 = img as u32;
            let present = vk::PresentInfoKHR::default()
                .wait_semaphores(std::slice::from_ref(&rf_sema))
                .swapchains(std::slice::from_ref(&self.swapchain))
                .image_indices(std::slice::from_ref(&img_u32));

            self.swapchain_loader
                .queue_present(self.present_queue, &present)
                .unwrap();
        }

        self.frame_idx = (fi + 1) % Self::MAX_FRAMES_IN_FLIGHT;
    }

    fn draw_sprites(&mut self, batch: &SpriteBatch) {
        if batch.instances.is_empty() {
            return;
        }
        assert!(batch.instances.len() <= MAX_SPRITES);
        let inst_size = std::mem::size_of::<SpriteInstance>() as vk::DeviceSize;
        let byte_count = batch.instances.len() as vk::DeviceSize * inst_size;
        unsafe {
            let ptr = self
                .device
                .map_memory(
                    self.instance_vbo_mem,
                    self.instance_cursor,
                    byte_count,
                    vk::MemoryMapFlags::empty(),
                )
                .unwrap() as *mut SpriteInstance;
            ptr.copy_from_nonoverlapping(batch.instances.as_ptr(), batch.instances.len());
            self.device.unmap_memory(self.instance_vbo_mem);
        }

        let cmd = self.cmds[self.frame_idx];
        let set = self.descriptor_sets[batch.tex.0 as usize];

        unsafe {
            self.device
                .cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);

            self.device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                std::slice::from_ref(&set),
                &[],
            );

            let buffers = [self.quad_vbo, self.instance_vbo];
            let offsets = [0, self.instance_cursor];
            self.device
                .cmd_bind_vertex_buffers(cmd, 0, &buffers, &offsets);

            self.device
                .cmd_draw(cmd, 4, batch.instances.len() as u32, 0, 0);
        }
        self.instance_cursor += byte_count;
    }

    fn init(app_name: &str, window: &Window) -> Result<Self, Self::Error> {
        let window_raw_handle = window.window_handle().unwrap().as_raw();
        let display_raw_handle = window.display_handle().unwrap().as_raw();
        let window_width = window.inner_size().width;
        let window_height = window.inner_size().height;
        unsafe {
            let entry = Entry::load().expect("Failed to load Vulkan entry point");

            let app_name = ffi::CString::new(app_name).expect("Empty app name");
            let engine_name = ffi::CString::new("Jester").expect("Empty engine name");

            let app_info = vk::ApplicationInfo::default()
                .application_name(&app_name)
                .engine_name(&engine_name)
                .engine_version(0)
                .api_version(API_VERSION_1_3)
                .application_version(vk::make_api_version(0, 0, 1, 0));

            let mut extension_names: Vec<*const i8> =
                enumerate_required_extensions(display_raw_handle)
                    .unwrap()
                    .to_vec();
            #[cfg(feature = "debug")]
            extension_names.push(debug_utils::NAME.as_ptr());
            extension_names.push(ash::khr::surface::NAME.as_ptr());
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            {
                extension_names.push(ash::khr::portability_enumeration::NAME.as_ptr());
                // Enabling this extension is a requirement when using `VK_KHR_portability_subset`
                extension_names.push(ash::khr::get_physical_device_properties2::NAME.as_ptr());
                extension_names.push(ash::ext::metal_surface::NAME.as_ptr());
            }

            let create_flags = if cfg!(any(target_os = "macos", target_os = "ios")) {
                vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
            } else {
                vk::InstanceCreateFlags::default()
            };

            #[cfg(feature = "debug")]
            let layers_names_raw = {
                let layer_names = [c"VK_LAYER_KHRONOS_validation"];
                let layers_names_raw: Vec<*const c_char> = layer_names
                    .iter()
                    .map(|raw_name| raw_name.as_ptr())
                    .collect();
                layers_names_raw
            };
            let create_info = vk::InstanceCreateInfo::default()
                .application_info(&app_info)
                .enabled_extension_names(&extension_names)
                .flags(create_flags);
            #[cfg(feature = "debug")]
            let create_info = create_info.enabled_layer_names(&layers_names_raw);

            let instance: Instance = entry
                .create_instance(&create_info, None)
                .expect("Instance creation error");

            let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                )
                .pfn_user_callback(Some(vulkan_debug_callback));

            #[cfg(feature = "debug")]
            let debug_utils_loader = debug_utils::Instance::new(&entry, &instance);
            #[cfg(feature = "debug")]
            let debug_call_back = debug_utils_loader
                .create_debug_utils_messenger(&debug_info, None)
                .unwrap();
            let surface = create_surface(
                &entry,
                &instance,
                display_raw_handle,
                window_raw_handle,
                None,
            )
            .unwrap();
            let pdevices = instance
                .enumerate_physical_devices()
                .expect("Physical device error");
            let surface_loader = surface::Instance::new(&entry, &instance);

            let (pdevice, queue_family_index) = pdevices
                .iter()
                .find_map(|pdevice| {
                    instance
                        .get_physical_device_queue_family_properties(*pdevice)
                        .iter()
                        .enumerate()
                        .find_map(|(index, info)| {
                            let supports_graphic_and_surface =
                                info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                                    && surface_loader
                                        .get_physical_device_surface_support(
                                            *pdevice,
                                            index as u32,
                                            surface,
                                        )
                                        .unwrap();
                            if supports_graphic_and_surface {
                                Some((*pdevice, index))
                            } else {
                                None
                            }
                        })
                })
                .expect("Couldn't find suitable device.");
            let queue_family_index = queue_family_index as u32;
            let device_extension_names_raw = [
                swapchain::NAME.as_ptr(),
                #[cfg(any(target_os = "macos", target_os = "ios"))]
                ash::khr::portability_subset::NAME.as_ptr(),
            ];
            let features = vk::PhysicalDeviceFeatures {
                shader_clip_distance: 1,
                ..Default::default()
            };
            let priorities = [1.0];

            let queue_info = vk::DeviceQueueCreateInfo::default()
                .queue_family_index(queue_family_index)
                .queue_priorities(&priorities);

            let device_create_info = vk::DeviceCreateInfo::default()
                .queue_create_infos(std::slice::from_ref(&queue_info))
                .enabled_extension_names(&device_extension_names_raw)
                .enabled_features(&features);

            let device: Device = instance
                .create_device(pdevice, &device_create_info, None)
                .unwrap();

            let present_queue = device.get_device_queue(queue_family_index, 0);

            let surface_format = surface_loader
                .get_physical_device_surface_formats(pdevice, surface)
                .unwrap()[0];

            let color_attach = vk::AttachmentDescription::default()
                .format(surface_format.format)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

            let color_ref = vk::AttachmentReference {
                attachment: 0,
                layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            };

            let subpass = vk::SubpassDescription::default()
                .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                .color_attachments(std::slice::from_ref(&color_ref));

            let rp_info = vk::RenderPassCreateInfo::default()
                .attachments(std::slice::from_ref(&color_attach))
                .subpasses(std::slice::from_ref(&subpass));

            let render_pass = device.create_render_pass(&rp_info, None)?;

            let surface_capabilities = surface_loader
                .get_physical_device_surface_capabilities(pdevice, surface)
                .unwrap();
            let mut desired_image_count = surface_capabilities.min_image_count + 1;
            if surface_capabilities.max_image_count > 0
                && desired_image_count > surface_capabilities.max_image_count
            {
                desired_image_count = surface_capabilities.max_image_count;
            }
            let surface_resolution = match surface_capabilities.current_extent.width {
                u32::MAX => vk::Extent2D {
                    width: window_width,
                    height: window_height,
                },
                _ => surface_capabilities.current_extent,
            };
            let pre_transform = if surface_capabilities
                .supported_transforms
                .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
            {
                vk::SurfaceTransformFlagsKHR::IDENTITY
            } else {
                surface_capabilities.current_transform
            };
            let present_modes = surface_loader
                .get_physical_device_surface_present_modes(pdevice, surface)
                .unwrap();
            let present_mode = present_modes
                .iter()
                .cloned()
                .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
                .unwrap_or(vk::PresentModeKHR::FIFO);
            let swapchain_loader = swapchain::Device::new(&instance, &device);

            let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
                .surface(surface)
                .min_image_count(desired_image_count)
                .image_color_space(surface_format.color_space)
                .image_format(surface_format.format)
                .image_extent(surface_resolution)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(pre_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(present_mode)
                .clipped(true)
                .image_array_layers(1);

            let swapchain = swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .unwrap();

            let pool_create_info = vk::CommandPoolCreateInfo::default()
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                .queue_family_index(queue_family_index);

            let pool = device.create_command_pool(&pool_create_info, None).unwrap();

            let cmd_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
                .command_buffer_count(VkBackend::MAX_FRAMES_IN_FLIGHT as u32)
                .command_pool(pool)
                .level(vk::CommandBufferLevel::PRIMARY);
            let cmd = device
                .allocate_command_buffers(&cmd_buffer_allocate_info)
                .unwrap();

            let present_images = swapchain_loader.get_swapchain_images(swapchain).unwrap();
            let present_image_views: Vec<vk::ImageView> = present_images
                .iter()
                .map(|&image| {
                    let create_view_info = vk::ImageViewCreateInfo::default()
                        .view_type(vk::ImageViewType::TYPE_2D)
                        .format(surface_format.format)
                        .components(vk::ComponentMapping {
                            r: vk::ComponentSwizzle::R,
                            g: vk::ComponentSwizzle::G,
                            b: vk::ComponentSwizzle::B,
                            a: vk::ComponentSwizzle::A,
                        })
                        .subresource_range(vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            base_mip_level: 0,
                            level_count: 1,
                            base_array_layer: 0,
                            layer_count: 1,
                        })
                        .image(image);
                    device.create_image_view(&create_view_info, None).unwrap()
                })
                .collect();
            let device_memory_properties = instance.get_physical_device_memory_properties(pdevice);

            let semaphore_create_info = vk::SemaphoreCreateInfo::default();

            let framebuffers: Vec<vk::Framebuffer> = present_image_views
                .iter()
                .map(|&view| {
                    let fb_info = vk::FramebufferCreateInfo::default()
                        .render_pass(render_pass)
                        .attachments(std::slice::from_ref(&view))
                        .width(surface_resolution.width)
                        .height(surface_resolution.height)
                        .layers(1);
                    device.create_framebuffer(&fb_info, None)
                })
                .collect::<Result<_, _>>()?;

            let mut image_available = [vk::Semaphore::null(); VkBackend::MAX_FRAMES_IN_FLIGHT];
            let render_finished = present_images
                .iter()
                .map(|_| device.create_semaphore(&semaphore_create_info, None))
                .collect::<Result<Vec<_>, _>>()?;
            let mut in_flight_fence = [vk::Fence::null(); VkBackend::MAX_FRAMES_IN_FLIGHT];

            for i in 0..VkBackend::MAX_FRAMES_IN_FLIGHT {
                image_available[i] = device.create_semaphore(&semaphore_create_info, None)?;
                in_flight_fence[i] = device.create_fence(
                    &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                    None,
                )?;
            }

            info!("Creating quad VBO");
            let quad_size =
                (std::mem::size_of::<QuadVertex>() * QUAD_VERTS.len()) as vk::DeviceSize;
            let (quad_vbo, quad_vbo_mem) = shaders::create_buffer(
                &device,
                &device_memory_properties,
                quad_size,
                vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            );

            info!("Creating quad staging buffer");
            {
                let (staging_buf, staging_mem) = shaders::create_buffer(
                    &device,
                    &device_memory_properties,
                    quad_size,
                    vk::BufferUsageFlags::TRANSFER_SRC,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                );

                let ptr =
                    device.map_memory(staging_mem, 0, quad_size, vk::MemoryMapFlags::empty())?
                        as *mut QuadVertex;
                ptr.copy_from_nonoverlapping(QUAD_VERTS.as_ptr(), QUAD_VERTS.len());
                device.unmap_memory(staging_mem);

                let alloc = vk::CommandBufferAllocateInfo::default()
                    .command_pool(pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1);
                let tmp_cmd = device.allocate_command_buffers(&alloc)?[0];
                let tmp_fence = device.create_fence(
                    &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                    None,
                )?;

                let region = vk::BufferCopy::default().size(quad_size);
                record_submit_commandbuffer(
                    &device,
                    tmp_cmd,
                    tmp_fence,
                    present_queue,
                    &[],
                    &[],
                    &[],
                    |d, c| {
                        d.cmd_copy_buffer(c, staging_buf, quad_vbo, std::slice::from_ref(&region));
                    },
                );
                device.wait_for_fences(&[tmp_fence], true, u64::MAX)?;
                device.destroy_fence(tmp_fence, None);
                device.free_command_buffers(pool, &[tmp_cmd]);
                device.destroy_buffer(staging_buf, None);
                device.free_memory(staging_mem, None);
            }
            info!("Creating instance VBO");
            let inst_size = (std::mem::size_of::<SpriteInstance>() * MAX_SPRITES) as vk::DeviceSize;
            let (instance_vbo, instance_vbo_mem) = shaders::create_buffer(
                &device,
                &device_memory_properties,
                inst_size,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );

            info!("Creating shader modules");
            let vert_mod =
                shaders::create_shader(&device, include_bytes!("shaders/sprite.vert.spv"));
            let frag_mod =
                shaders::create_shader(&device, include_bytes!("shaders/sprite.frag.spv"));

            info!("Creating pipeline layout");
            let set_layout_binding = vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT);
            let desc_set_layout = device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::default()
                    .bindings(std::slice::from_ref(&set_layout_binding)),
                None,
            )?;

            let pc_range = vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .offset(0)
                .size(std::mem::size_of::<[f32; 2]>() as u32);

            let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(std::slice::from_ref(&desc_set_layout))
                .push_constant_ranges(std::slice::from_ref(&pc_range));
            let pipeline_layout = device.create_pipeline_layout(&pipeline_layout_info, None)?;

            let desc_pool_size = vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count((MAX_TEXTURES * MAX_TEXTURES) as u32);
            let desc_pool = device.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::default()
                    .max_sets(MAX_TEXTURES as u32)
                    .pool_sizes(std::slice::from_ref(&desc_pool_size)),
                None,
            )?;

            let binding_descriptions = [
                vk::VertexInputBindingDescription::default() // binding 0: quad verts
                    .binding(0)
                    .stride(std::mem::size_of::<QuadVertex>() as u32)
                    .input_rate(vk::VertexInputRate::VERTEX),
                vk::VertexInputBindingDescription::default() // binding 1: per instance
                    .binding(1)
                    .stride(std::mem::size_of::<SpriteInstance>() as u32)
                    .input_rate(vk::VertexInputRate::INSTANCE),
            ];

            let attribute_descriptions = [
                // binding 0
                vk::VertexInputAttributeDescription::default()
                    .binding(0)
                    .location(0)
                    .format(vk::Format::R32G32_SFLOAT)
                    .offset(0),
                vk::VertexInputAttributeDescription::default()
                    .binding(0)
                    .location(1)
                    .format(vk::Format::R32G32_SFLOAT)
                    .offset(8),
                // binding 1
                vk::VertexInputAttributeDescription::default()
                    .binding(1)
                    .location(2)
                    .format(vk::Format::R32G32B32A32_SFLOAT)
                    .offset(0),
                vk::VertexInputAttributeDescription::default()
                    .binding(1)
                    .location(3)
                    .format(vk::Format::R32G32B32A32_SFLOAT)
                    .offset(16),
            ];

            let vertex_state = vk::PipelineVertexInputStateCreateInfo::default()
                .vertex_binding_descriptions(&binding_descriptions)
                .vertex_attribute_descriptions(&attribute_descriptions);

            let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
                .topology(vk::PrimitiveTopology::TRIANGLE_STRIP)
                .primitive_restart_enable(false);

            let viewport_state = vk::PipelineViewportStateCreateInfo::default()
                .viewport_count(1)
                .scissor_count(1);

            let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
            let dynamic_state =
                vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

            let raster = vk::PipelineRasterizationStateCreateInfo::default()
                .polygon_mode(vk::PolygonMode::FILL)
                .cull_mode(vk::CullModeFlags::NONE)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                .line_width(1.0);
            let multisample = vk::PipelineMultisampleStateCreateInfo::default()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1);
            let colour_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_write_mask(
                    vk::ColorComponentFlags::R
                        | vk::ColorComponentFlags::G
                        | vk::ColorComponentFlags::B
                        | vk::ColorComponentFlags::A,
                );
            let colour_blend = vk::PipelineColorBlendStateCreateInfo::default()
                .attachments(std::slice::from_ref(&colour_blend_attachment));

            let shader_entry = std::ffi::CString::new("main").unwrap();
            let stages = [
                vk::PipelineShaderStageCreateInfo::default()
                    .module(vert_mod)
                    .name(&shader_entry)
                    .stage(vk::ShaderStageFlags::VERTEX),
                vk::PipelineShaderStageCreateInfo::default()
                    .module(frag_mod)
                    .name(&shader_entry)
                    .stage(vk::ShaderStageFlags::FRAGMENT),
            ];

            let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
                .stages(&stages)
                .vertex_input_state(&vertex_state)
                .input_assembly_state(&input_assembly)
                .viewport_state(&viewport_state)
                .dynamic_state(&dynamic_state)
                .rasterization_state(&raster)
                .multisample_state(&multisample)
                .color_blend_state(&colour_blend)
                .layout(pipeline_layout)
                .render_pass(render_pass)
                .subpass(0);

            info!("Creating pipeline");
            let pipeline = device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    std::slice::from_ref(&pipeline_info),
                    None,
                )
                .map_err(|(_, e)| e)?[0];

            info!("Destroying shader modules");
            device.destroy_shader_module(vert_mod, None);
            device.destroy_shader_module(frag_mod, None);

            Ok(Self {
                entry,
                instance,
                device,
                queue_family_index,
                pdevice,
                device_memory_properties,
                surface_loader,
                surface_format,
                present_queue,
                surface_resolution,
                swapchain_loader,
                swapchain,
                present_images,
                present_image_views,
                pool,
                surface,
                #[cfg(feature = "debug")]
                debug_call_back,
                #[cfg(feature = "debug")]
                debug_utils_loader,
                render_pass,
                framebuffers,
                current_img: 0,
                image_available,
                render_finished,
                in_flight_fence,
                frame_idx: 0,
                cmds: cmd,
                swapchain_rebuild: false,
                pipeline,
                pipeline_layout,
                quad_vbo,
                quad_vbo_mem,
                instance_vbo,
                instance_vbo_mem,
                desc_set_layout,
                desc_pool,
                descriptor_sets: Vec::new(),

                images: Vec::new(),
                image_mem: Vec::new(),
                image_views: Vec::new(),
                samplers: Vec::new(),
                instance_cursor: 0,
            })
        }
    }
}

impl Drop for VkBackend {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().ok();

            for ((&img, &mem), (&view, &samp)) in self
                .images
                .iter()
                .zip(&self.image_mem)
                .zip(self.image_views.iter().zip(&self.samplers))
            {
                self.device.destroy_sampler(samp, None);
                self.device.destroy_image_view(view, None);
                self.device.destroy_image(img, None);
                self.device.free_memory(mem, None);
            }

            self.device.destroy_descriptor_pool(self.desc_pool, None);
            self.device
                .destroy_descriptor_set_layout(self.desc_set_layout, None);

            self.device.destroy_buffer(self.quad_vbo, None);
            self.device.free_memory(self.quad_vbo_mem, None);
            self.device.destroy_buffer(self.instance_vbo, None);
            self.device.free_memory(self.instance_vbo_mem, None);

            self.device.destroy_pipeline(self.pipeline, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);

            for &fb in &self.framebuffers {
                self.device.destroy_framebuffer(fb, None);
            }
            self.device.destroy_render_pass(self.render_pass, None);
            for &view in &self.present_image_views {
                self.device.destroy_image_view(view, None);
            }
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);

            for &s in &self.image_available {
                self.device.destroy_semaphore(s, None);
            }
            for &s in &self.render_finished {
                self.device.destroy_semaphore(s, None);
            }
            for &f in &self.in_flight_fence {
                self.device.destroy_fence(f, None);
            }

            self.device.destroy_command_pool(self.pool, None);

            self.surface_loader.destroy_surface(self.surface, None);
            self.device.destroy_device(None);

            #[cfg(feature = "debug")]
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_call_back, None);

            self.instance.destroy_instance(None);
        }
    }
}

mod shaders {
    use crate::utils::find_memorytype_index;
    use ash::{vk, Device};

    pub fn create_buffer(
        device: &Device,
        mem_props: &vk::PhysicalDeviceMemoryProperties,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        props: vk::MemoryPropertyFlags,
    ) -> (vk::Buffer, vk::DeviceMemory) {
        let info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buffer = unsafe { device.create_buffer(&info, None).unwrap() };

        let req = unsafe { device.get_buffer_memory_requirements(buffer) };
        let type_index = find_memorytype_index(&req, mem_props, props)
            .expect("No suitable memory type for buffer");
        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(req.size)
            .memory_type_index(type_index);
        let memory = unsafe { device.allocate_memory(&alloc_info, None).unwrap() };
        unsafe { device.bind_buffer_memory(buffer, memory, 0).unwrap() };

        (buffer, memory)
    }
    pub fn create_shader(device: &Device, bytes: &[u8]) -> vk::ShaderModule {
        let (prefix, code, _) = unsafe { bytes.align_to::<u32>() };
        assert!(prefix.is_empty(), "SPIR-V must be 4-byte aligned");
        let info = vk::ShaderModuleCreateInfo::default().code(code);
        unsafe { device.create_shader_module(&info, None).unwrap() }
    }
}
