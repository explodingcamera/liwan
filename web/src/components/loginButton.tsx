import { getUsername } from "../api/utils";
import { mutateLogout } from "../api";

export const LoginButton = () => {
	const username = getUsername();
	if (!username)
		return (
			<li>
				<a href="/login">
					<button type="button" className="outline secondary">
						Login
					</button>
				</a>
				&nbsp;&nbsp;
			</li>
		);

	return (
		<>
			<li>{username}</li>
			<li>
				<button
					className="outline secondary"
					onClick={() => {
						mutateLogout().then(() => {
							window.location.href = "/";
						});
					}}
					type="button"
				>
					Logout
				</button>
			</li>
		</>
	);
};
