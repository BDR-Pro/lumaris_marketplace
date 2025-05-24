
        // Particle animation
        function createParticle() {
            const particle = document.createElement('div');
            particle.className = 'particle';
            particle.style.left = Math.random() * 100 + '%';
            particle.style.animationDuration = (Math.random() * 3 + 3) + 's';
            particle.style.animationDelay = Math.random() * 2 + 's';
            document.getElementById('particles').appendChild(particle);

            setTimeout(() => {
                particle.remove();
            }, 6000);
        }

        // Create particles periodically
        setInterval(createParticle, 200);

        // Animate stats numbers
        function animateNumber(elementId, target) {
            const element = document.getElementById(elementId);
            if (!element) return;
            
            let current = 0;
            const increment = target / 100;
            const timer = setInterval(() => {
                current += increment;
                if (current >= target) {
                    current = target;
                    clearInterval(timer);
                }
                element.textContent = Math.floor(current);
            }, 20);
        }

        // Initialize animations when page loads
        document.addEventListener('DOMContentLoaded', function() {
            // Animate stats if they exist
            const activeNodes = parseInt(document.getElementById('activeNodes')?.textContent) || 0;
            const totalJobs = parseInt(document.getElementById('totalJobs')?.textContent) || 0;
            const completedJobs = parseInt(document.getElementById('completedJobs')?.textContent) || 0;

            setTimeout(() => {
                animateNumber('activeNodes', activeNodes);
                animateNumber('totalJobs', totalJobs);
                animateNumber('completedJobs', completedJobs);
            }, 500);
        });

        // Mobile navigation toggle
        function toggleMobileNav() {
            const navLinks = document.querySelector('.nav-links');
            navLinks.classList.toggle('active');
        }

        // Smooth scrolling for anchor links
        document.querySelectorAll('a[href^="#"]').forEach(anchor => {
            anchor.addEventListener('click', function (e) {
                e.preventDefault();
                const target = document.querySelector(this.getAttribute('href'));
                if (target) {
                    target.scrollIntoView({
                        behavior: 'smooth',
                        block: 'start'
                    });
                }
            });
        });

        // WebSocket connection for real-time updates (if on dashboard)
        if (window.location.pathname.includes('dashboard')) {
            // use WEBSOCKET_URL from django settings env variable LUMARIS_WS_URL
            
            const wsUrl = window.LUMARIS_WS_URL;
            try {
                const ws = new WebSocket(wsUrl);
                
                ws.onopen = function() {
                    console.log('Connected to Lumaris WebSocket');
                };
                
                ws.onmessage = function(event) {
                    const data = JSON.parse(event.data);
                    // Handle real-time updates here
                    console.log('Received update:', data);
                };
                
                ws.onerror = function(error) {
                    console.log('WebSocket error:', error);
                };
            } catch (error) {
                console.log('WebSocket connection failed:', error);
            }
        }