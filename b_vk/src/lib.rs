use ash::{
    ext::{debug_utils, metal_surface},
    khr::{
        android_surface, surface, swapchain, wayland_surface, win32_surface, xcb_surface,
        xlib_surface,
    },
    prelude::VkResult,
    vk::{self, API_VERSION_1_3},
    Device, Entry, Instance,
};
use jester_core::{Backend, SpriteBatch};
use std::{borrow::Cow, ffi, os::raw::c_char};
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle},
    window::Window,
};

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
    pub setup_command_buffer: vk::CommandBuffer,
    pub cmds: Vec<vk::CommandBuffer>,

    pub draw_commands_reuse_fence: vk::Fence,
    pub setup_commands_reuse_fence: vk::Fence,

    pub render_pass: vk::RenderPass,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub current_img: usize,
    pub image_available: [vk::Semaphore; Self::MAX_FRAMES_IN_FLIGHT],
    pub render_finished: Vec<vk::Semaphore>,
    pub in_flight_fence: [vk::Fence; Self::MAX_FRAMES_IN_FLIGHT],

    pub frame_idx: usize,

    // misc
    pub swapchain_rebuild: bool,
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

            self.device.cmd_end_render_pass(cmd);
            self.device.end_command_buffer(cmd).unwrap();
        }
    }

    fn end_frame(&mut self) {
        let fi = self.frame_idx;
        let img = self.current_img;
        let cmd = self.cmds[fi];
        let rf_sema = self.render_finished[img];

        let submit = vk::SubmitInfo::default()
            .wait_semaphores(std::slice::from_ref(&self.image_available[fi]))
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .command_buffers(std::slice::from_ref(&cmd))
            .signal_semaphores(std::slice::from_ref(&rf_sema));

        unsafe {
            self.device
                .queue_submit(self.present_queue, &[submit], self.in_flight_fence[fi])
                .expect("queue submit failed.");

            let img_u32 = img as u32;
            let present = vk::PresentInfoKHR::default()
                .wait_semaphores(std::slice::from_ref(&rf_sema))
                .swapchains(std::slice::from_ref(&self.swapchain))
                .image_indices(std::slice::from_ref(&img_u32));

            self.swapchain_loader
                .queue_present(self.present_queue, &present)
                .expect("present failed.");
        }

        self.frame_idx = (fi + 1) % VkBackend::MAX_FRAMES_IN_FLIGHT;
    }

    fn draw_sprites(&mut self, batch: &SpriteBatch) {
        todo!()
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

            let setup_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
                .command_buffer_count(1)
                .command_pool(pool)
                .level(vk::CommandBufferLevel::PRIMARY);

            let command_buffers = device
                .allocate_command_buffers(&setup_buffer_allocate_info)
                .unwrap();
            let setup_command_buffer = command_buffers[0];

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
            let depth_image_create_info = vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(vk::Format::D16_UNORM)
                .extent(surface_resolution.into())
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);

            let depth_image = device.create_image(&depth_image_create_info, None).unwrap();
            let depth_image_memory_req = device.get_image_memory_requirements(depth_image);
            let depth_image_memory_index = find_memorytype_index(
                &depth_image_memory_req,
                &device_memory_properties,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )
            .expect("Unable to find suitable memory index for depth image.");

            let depth_image_allocate_info = vk::MemoryAllocateInfo::default()
                .allocation_size(depth_image_memory_req.size)
                .memory_type_index(depth_image_memory_index);

            let depth_image_memory = device
                .allocate_memory(&depth_image_allocate_info, None)
                .unwrap();

            device
                .bind_image_memory(depth_image, depth_image_memory, 0)
                .expect("Unable to bind depth image memory");

            let fence_create_info =
                vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

            let draw_commands_reuse_fence = device
                .create_fence(&fence_create_info, None)
                .expect("Create fence failed.");
            let setup_commands_reuse_fence = device
                .create_fence(&fence_create_info, None)
                .expect("Create fence failed.");

            record_submit_commandbuffer(
                &device,
                setup_command_buffer,
                setup_commands_reuse_fence,
                present_queue,
                &[],
                &[],
                &[],
                |device, setup_command_buffer| {
                    let layout_transition_barriers = vk::ImageMemoryBarrier::default()
                        .image(depth_image)
                        .dst_access_mask(
                            vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                        )
                        .new_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                        .old_layout(vk::ImageLayout::UNDEFINED)
                        .subresource_range(
                            vk::ImageSubresourceRange::default()
                                .aspect_mask(vk::ImageAspectFlags::DEPTH)
                                .layer_count(1)
                                .level_count(1),
                        );

                    device.cmd_pipeline_barrier(
                        setup_command_buffer,
                        vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                        vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[layout_transition_barriers],
                    );
                },
            );

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
                setup_command_buffer,
                // present_complete_semaphore,
                // rendering_complete_semaphore,
                draw_commands_reuse_fence,
                setup_commands_reuse_fence,
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
            })
        }
    }
}

impl Drop for VkBackend {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
            // self.device
            //     .destroy_semaphore(self.present_complete_semaphore, None);
            // self.device
            //     .destroy_semaphore(self.rendering_complete_semaphore, None);
            self.device
                .destroy_fence(self.draw_commands_reuse_fence, None);
            self.device
                .destroy_fence(self.setup_commands_reuse_fence, None);
            for &semaphore in self.image_available.iter() {
                self.device.destroy_semaphore(semaphore, None);
            }
            for &semaphore in self.render_finished.iter() {
                self.device.destroy_semaphore(semaphore, None);
            }
            for &fence in self.in_flight_fence.iter() {
                self.device.destroy_fence(fence, None);
            }
            for &image_view in self.present_image_views.iter() {
                self.device.destroy_image_view(image_view, None);
            }
            self.device.destroy_command_pool(self.pool, None);
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_call_back, None);
            self.instance.destroy_instance(None);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn record_submit_commandbuffer<F: FnOnce(&Device, vk::CommandBuffer)>(
    device: &Device,
    command_buffer: vk::CommandBuffer,
    command_buffer_reuse_fence: vk::Fence,
    submit_queue: vk::Queue,
    wait_mask: &[vk::PipelineStageFlags],
    wait_semaphores: &[vk::Semaphore],
    signal_semaphores: &[vk::Semaphore],
    f: F,
) {
    unsafe {
        device
            .wait_for_fences(&[command_buffer_reuse_fence], true, u64::MAX)
            .expect("Wait for fence failed.");

        device
            .reset_fences(&[command_buffer_reuse_fence])
            .expect("Reset fences failed.");

        device
            .reset_command_buffer(
                command_buffer,
                vk::CommandBufferResetFlags::RELEASE_RESOURCES,
            )
            .expect("Reset command buffer failed.");

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        device
            .begin_command_buffer(command_buffer, &command_buffer_begin_info)
            .expect("Begin commandbuffer");
        f(device, command_buffer);
        device
            .end_command_buffer(command_buffer)
            .expect("End commandbuffer");

        let command_buffers = vec![command_buffer];

        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_mask)
            .command_buffers(&command_buffers)
            .signal_semaphores(signal_semaphores);

        device
            .queue_submit(submit_queue, &[submit_info], command_buffer_reuse_fence)
            .expect("queue submit failed.");
    }
}

pub fn find_memorytype_index(
    memory_req: &vk::MemoryRequirements,
    memory_prop: &vk::PhysicalDeviceMemoryProperties,
    flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    memory_prop.memory_types[..memory_prop.memory_type_count as _]
        .iter()
        .enumerate()
        .find(|(index, memory_type)| {
            (1 << index) & memory_req.memory_type_bits != 0
                && memory_type.property_flags & flags == flags
        })
        .map(|(index, _memory_type)| index as _)
}

#[allow(clippy::missing_safety_doc)]
pub unsafe fn create_surface(
    entry: &Entry,
    instance: &Instance,
    display_handle: RawDisplayHandle,
    window_handle: RawWindowHandle,
    allocation_callbacks: Option<&vk::AllocationCallbacks>,
) -> VkResult<vk::SurfaceKHR> {
    unsafe {
        match (display_handle, window_handle) {
            (RawDisplayHandle::Windows(_), RawWindowHandle::Win32(window)) => {
                let surface_desc = vk::Win32SurfaceCreateInfoKHR::default()
                    .hwnd(window.hwnd.get())
                    .hinstance(
                        window
                            .hinstance
                            .ok_or(vk::Result::ERROR_INITIALIZATION_FAILED)?
                            .get(),
                    );
                let surface_fn = win32_surface::Instance::new(entry, instance);
                surface_fn.create_win32_surface(&surface_desc, allocation_callbacks)
            }

            (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(window)) => {
                let surface_desc = vk::WaylandSurfaceCreateInfoKHR::default()
                    .display(display.display.as_ptr())
                    .surface(window.surface.as_ptr());
                let surface_fn = wayland_surface::Instance::new(entry, instance);
                surface_fn.create_wayland_surface(&surface_desc, allocation_callbacks)
            }

            (RawDisplayHandle::Xlib(display), RawWindowHandle::Xlib(window)) => {
                let surface_desc = vk::XlibSurfaceCreateInfoKHR::default()
                    .dpy(
                        display
                            .display
                            .ok_or(vk::Result::ERROR_INITIALIZATION_FAILED)?
                            .as_ptr(),
                    )
                    .window(window.window);
                let surface_fn = xlib_surface::Instance::new(entry, instance);
                surface_fn.create_xlib_surface(&surface_desc, allocation_callbacks)
            }

            (RawDisplayHandle::Xcb(display), RawWindowHandle::Xcb(window)) => {
                let surface_desc = vk::XcbSurfaceCreateInfoKHR::default()
                    .connection(
                        display
                            .connection
                            .ok_or(vk::Result::ERROR_INITIALIZATION_FAILED)?
                            .as_ptr(),
                    )
                    .window(window.window.get());
                let surface_fn = xcb_surface::Instance::new(entry, instance);
                surface_fn.create_xcb_surface(&surface_desc, allocation_callbacks)
            }

            (RawDisplayHandle::Android(_), RawWindowHandle::AndroidNdk(window)) => {
                let surface_desc = vk::AndroidSurfaceCreateInfoKHR::default()
                    .window(window.a_native_window.as_ptr());
                let surface_fn = android_surface::Instance::new(entry, instance);
                surface_fn.create_android_surface(&surface_desc, allocation_callbacks)
            }

            #[cfg(target_os = "macos")]
            (RawDisplayHandle::AppKit(_), RawWindowHandle::AppKit(window)) => {
                use raw_window_metal::{appkit, Layer};

                let layer = match appkit::metal_layer_from_handle(window) {
                    Layer::Existing(layer) | Layer::Allocated(layer) => layer.cast(),
                };

                let surface_desc = vk::MetalSurfaceCreateInfoEXT::default().layer(&*layer);
                let surface_fn = metal_surface::Instance::new(entry, instance);
                surface_fn.create_metal_surface(&surface_desc, allocation_callbacks)
            }

            #[cfg(target_os = "ios")]
            (RawDisplayHandle::UiKit(_), RawWindowHandle::UiKit(window)) => {
                use raw_window_metal::{uikit, Layer};

                let layer = match uikit::metal_layer_from_handle(window) {
                    Layer::Existing(layer) | Layer::Allocated(layer) => layer.cast(),
                };

                let surface_desc = vk::MetalSurfaceCreateInfoEXT::default().layer(&*layer);
                let surface_fn = metal_surface::Instance::new(entry, instance);
                surface_fn.create_metal_surface(&surface_desc, allocation_callbacks)
            }

            _ => Err(vk::Result::ERROR_EXTENSION_NOT_PRESENT),
        }
    }
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = unsafe { *p_callback_data };
    let message_id_number = callback_data.message_id_number;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        unsafe { ffi::CStr::from_ptr(callback_data.p_message_id_name) }.to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        unsafe { ffi::CStr::from_ptr(callback_data.p_message) }.to_string_lossy()
    };

    println!(
        "{message_severity:?}:\n{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n",
    );

    vk::FALSE
}

pub fn enumerate_required_extensions(
    display_handle: RawDisplayHandle,
) -> VkResult<&'static [*const c_char]> {
    let extensions = match display_handle {
        RawDisplayHandle::Windows(_) => {
            const WINDOWS_EXTS: [*const c_char; 2] =
                [surface::NAME.as_ptr(), win32_surface::NAME.as_ptr()];
            &WINDOWS_EXTS
        }

        RawDisplayHandle::Wayland(_) => {
            const WAYLAND_EXTS: [*const c_char; 2] =
                [surface::NAME.as_ptr(), wayland_surface::NAME.as_ptr()];
            &WAYLAND_EXTS
        }

        RawDisplayHandle::Xlib(_) => {
            const XLIB_EXTS: [*const c_char; 2] =
                [surface::NAME.as_ptr(), xlib_surface::NAME.as_ptr()];
            &XLIB_EXTS
        }

        RawDisplayHandle::Xcb(_) => {
            const XCB_EXTS: [*const c_char; 2] =
                [surface::NAME.as_ptr(), xcb_surface::NAME.as_ptr()];
            &XCB_EXTS
        }

        RawDisplayHandle::Android(_) => {
            const ANDROID_EXTS: [*const c_char; 2] =
                [surface::NAME.as_ptr(), android_surface::NAME.as_ptr()];
            &ANDROID_EXTS
        }

        RawDisplayHandle::AppKit(_) | RawDisplayHandle::UiKit(_) => {
            const METAL_EXTS: [*const c_char; 2] =
                [surface::NAME.as_ptr(), metal_surface::NAME.as_ptr()];
            &METAL_EXTS
        }

        _ => return Err(vk::Result::ERROR_EXTENSION_NOT_PRESENT),
    };

    Ok(extensions)
}
