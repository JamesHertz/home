const form = document.querySelector('form');
form.addEventListener('submit', async e => {
	e.preventDefault()

	const credentials = {
		login:    form.login.value,
		password: form.password.value,
	};

	try {
		const response = await fetch("/api/login", {
			method: "POST",
			headers: {
				"Content-Type": "application/json",
			},
			body: JSON.stringify(credentials)
		});

		if(response.ok) {
			const { redirectUrl }  = await response.json();
			document.location.href = redirectUrl;
		} else {
			form.login.value    = '';
			form.password.value = '';
			alert("invalid credentials");
		}

		console.log(response)
	} catch(e){
		alert("error trying to reach the server");
		console.log(e);
	}

})
