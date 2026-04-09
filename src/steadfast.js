(()=>{
	if(document.__fixi_mo) return;
	document.__fixi_mo = new MutationObserver((recs)=>recs.forEach((r)=>r.type === "childList" && r.addedNodes.forEach((n)=>process(n))))
	let send = (elt, type, detail, bub)=>elt.dispatchEvent(new CustomEvent("sf:" + type, {detail, cancelable:true, bubbles:bub !== false, composed:true}))
	let attr = (elt, name, defaultVal)=>elt.getAttribute(name) || defaultVal
	let ignore = (elt)=>elt.closest("[sf-ignore]") != null
	let load = (elt)=>elt.closest("[sf-load]") != null
	let init = (elt)=>{
		let options = {}
		if (elt.__fixi || ignore(elt) || !send(elt, "init", {options})) return
		elt.__fixi = async(evt)=>{
			let reqs = elt.__fixi.requests ||= new Set()
			let form = elt.form || elt.closest("form")
			// let body = new FormData(form ?? undefined, evt.submitter)
			let body = new FormData(form ?? undefined, evt.submitter)
			if (!form && elt.name) body.append(elt.name, elt.value)
			let ac = new AbortController()
			let cfg = {
				trigger:evt,
				action:attr(elt, "sf-action"),
				method:attr(elt, "sf-method", "GET").toUpperCase(),
				target:document.querySelector(attr(elt, "sf-target")) ?? elt,
				swap:attr(elt, "sf-swap", "outerHTML"),
				body,
				drop:reqs.size,
				headers:{"sf-request":"true", "sf-tz":Intl.DateTimeFormat().resolvedOptions().timeZone},
				abort:ac.abort.bind(ac),
				signal:ac.signal,
				preventTrigger:true,
				transition:document.startViewTransition?.bind(document),
				fetch:fetch.bind(window),
				scroll_to:document.querySelector(attr(elt, "sf-scroll-to", "#notarealid")),
			}
			if (/TOGGLE CSS/.test(cfg.method)&&/CLASS/.test(cfg.swap.toUpperCase())) {
				cfg.target.classList.toggle(cfg.action);
				return;
			}
			if (/REMOVE CSS/.test(cfg.method)&&/CLASS/.test(cfg.swap.toUpperCase())) {
				cfg.target.classList.remove(cfg.action);
				return;
			}
			let go = send(elt, "config", {cfg, requests:reqs})
			if (cfg.preventTrigger) evt.preventDefault()
			if (!go || cfg.drop) return
			if (/GET|DELETE/.test(cfg.method)){
				let params = new URLSearchParams(cfg.body)
				if (params.size)
					cfg.action += (/\?/.test(cfg.action) ? "&" : "?") + params
				cfg.body = null
			}
			reqs.add(cfg)
			try {
				if (cfg.confirm){
					let result = await cfg.confirm()
					if (!result) return
				}
				if (!send(elt, "before", {cfg, requests:reqs})) return
				cfg.response = await cfg.fetch(cfg.action, cfg)
				cfg.text = await cfg.response.text()
				if (!send(elt, "after", {cfg})) return
			} catch(error) {
				send(elt, "error", {cfg, error})
				return
			} finally {
				reqs.delete(cfg)
				send(elt, "finally", {cfg})
			}
			let doSwap = ()=>{
				if (cfg.swap instanceof Function)
					return cfg.swap(cfg)
				else if (/(before|after)(begin|end)/.test(cfg.swap))
					cfg.target.insertAdjacentHTML(cfg.swap, cfg.text)
				else if(cfg.swap in cfg.target)
					cfg.target[cfg.swap] = cfg.text
				else if(cfg.swap !== 'none') throw cfg.swap
			}
			if (cfg.transition)
				await cfg.transition(doSwap).finished
			else
				await doSwap()
			send(elt, "swapped", {cfg})
			if (!document.contains(elt)) send(document, "swapped", {cfg})
			if (cfg.scroll_to) {
				cfg.scroll_to.scrollIntoView({ behavior: 'smooth', block: 'start' });
			}
		}
		elt.__fixi.evt = attr(elt, "sf-trigger", elt.matches("form") ? "submit" : elt.matches("input:not([type=button]),select,textarea") ? "change" : "click")
		elt.__fixi.evt=="load" ? elt.__fixi({preventDefault: function() {}}) : elt.addEventListener(elt.__fixi.evt, elt.__fixi, options)
		send(elt, "inited", {}, false)
	}
	let process = (n)=>{
    if (n.shadowRoot) {
      document.__fixi_mo.observe(n.shadowRoot, {
        childList: true,
        subtree: true
      });
    }
    if (n.matches){
			if (ignore(n)) return
			if (n.matches("[sf-action]")) init(n)
		}
		if(n.querySelectorAll) n.querySelectorAll("[sf-action]").forEach(init)
	}
	document.addEventListener("sf:process", (evt)=>process(evt.target))
	document.addEventListener("DOMContentLoaded", ()=>{
		document.__fixi_mo.observe(document.documentElement, {childList:true, subtree:true})
		process(document.body)
	})
})()