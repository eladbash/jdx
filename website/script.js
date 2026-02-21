// ── Copy to clipboard ───────────────────────────────
document.querySelectorAll('.copy-btn').forEach(btn => {
  btn.addEventListener('click', () => {
    const text = btn.dataset.copy;
    navigator.clipboard.writeText(text).then(() => {
      btn.classList.add('copied');
      const orig = btn.innerHTML;
      btn.innerHTML = '<span>&#10003;</span> Copied';
      setTimeout(() => {
        btn.classList.remove('copied');
        btn.innerHTML = orig;
      }, 1500);
    });
  });
});

// ── Install method pill switcher ─────────────────────
const installCmd = document.getElementById('install-cmd');
const installCopyBtn = document.getElementById('install-copy-btn');
const copyIcon = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/></svg>';

document.querySelectorAll('.install-pill').forEach(pill => {
  pill.addEventListener('click', () => {
    document.querySelectorAll('.install-pill').forEach(p => p.classList.remove('active'));
    pill.classList.add('active');
    installCmd.innerHTML = pill.dataset.html;
    installCopyBtn.dataset.copy = pill.dataset.cmd;
    installCopyBtn.innerHTML = copyIcon;
    installCopyBtn.classList.remove('copied');
  });
});

// ── Smooth scroll for nav anchors ───────────────────
document.querySelectorAll('a[href^="#"]').forEach(anchor => {
  anchor.addEventListener('click', e => {
    e.preventDefault();
    const target = document.querySelector(anchor.getAttribute('href'));
    if (target) {
      target.scrollIntoView({ behavior: 'smooth', block: 'start' });
      // Close mobile menu if open
      document.getElementById('mobile-menu')?.classList.remove('open');
    }
  });
});

// ── Navbar background on scroll ─────────────────────
const navbar = document.getElementById('navbar');
window.addEventListener('scroll', () => {
  navbar.classList.toggle('scrolled', window.scrollY > 40);
}, { passive: true });

// ── Scroll-triggered fade-in animations ─────────────
const observer = new IntersectionObserver(
  entries => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        entry.target.classList.add('visible');
      }
    });
  },
  { threshold: 0.1, rootMargin: '0px 0px -40px 0px' }
);

document.querySelectorAll('.fade-in').forEach(el => observer.observe(el));

// ── Mobile hamburger nav toggle ─────────────────────
const hamburger = document.getElementById('hamburger');
const mobileMenu = document.getElementById('mobile-menu');
const mobileClose = document.getElementById('mobile-close');

hamburger?.addEventListener('click', () => {
  mobileMenu.classList.add('open');
});

mobileClose?.addEventListener('click', () => {
  mobileMenu.classList.remove('open');
});
